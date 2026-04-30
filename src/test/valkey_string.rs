use crate::raw;
use libc::{c_int, c_longlong};
use std::cmp::Ordering;
use std::os::raw::c_char;
use std::slice;

// Minimal in-memory stand-in for RedisModuleString used by unit tests that run
// without a Valkey server or loaded raw API table.
#[derive(Debug)]
struct TestValkeyString {
    // Keep bytes, not String, because RedisModuleString can contain arbitrary
    // binary data.
    data: Vec<u8>,
    // The real RedisModuleString is ref-counted. A small manual counter is
    // enough for the unit-test operations this shim supports.
    ref_count: usize,
}

impl TestValkeyString {
    fn from_slice(data: &[u8]) -> *mut raw::RedisModuleString {
        let test_string = Self {
            data: data.to_vec(),
            ref_count: 1,
        };
        let boxed = Box::new(test_string);

        // The production API exposes RedisModuleString as an opaque pointer, so
        // tests cast this boxed shim through the same opaque raw type.
        let raw_test_string = Box::into_raw(boxed);
        raw_test_string.cast::<raw::RedisModuleString>()
    }
}

fn as_test_string<'a>(s: *const raw::RedisModuleString) -> &'a TestValkeyString {
    // All pointers handled by these callbacks are created by TestValkeyString::from_slice.
    let raw_test_string = s.cast::<TestValkeyString>();
    unsafe { &*raw_test_string }
}

fn as_test_string_mut<'a>(s: *mut raw::RedisModuleString) -> &'a mut TestValkeyString {
    // Mutable callbacks receive the same opaque pointer shape as production,
    // but the allocation behind it is our boxed TestValkeyString.
    let raw_test_string = s.cast::<TestValkeyString>();
    unsafe { &mut *raw_test_string }
}

fn buffer_as_slice<'a>(ptr: *const c_char, len: usize) -> &'a [u8] {
    // Redis passes buffers as pointer + length. Avoid building a slice from a
    // possibly-null pointer when the length is zero.
    if len == 0 {
        return &[];
    }

    let ptr = ptr.cast::<u8>();
    unsafe { slice::from_raw_parts(ptr, len) }
}

pub(super) extern "C" fn test_create_string(
    _ctx: *mut raw::RedisModuleCtx,
    ptr: *const c_char,
    len: usize,
) -> *mut raw::RedisModuleString {
    // RedisModule_CreateString copies the provided buffer; mirror that behavior
    // so callers do not depend on the source slice lifetime.
    let data = buffer_as_slice(ptr, len);
    TestValkeyString::from_slice(data)
}

pub(super) extern "C" fn test_create_string_from_string(
    _ctx: *mut raw::RedisModuleCtx,
    s: *const raw::RedisModuleString,
) -> *mut raw::RedisModuleString {
    // RedisModule_CreateStringFromString creates an independent string object,
    // so clone the underlying bytes instead of sharing the ref count.
    let test_string = as_test_string(s);
    TestValkeyString::from_slice(&test_string.data)
}

pub(super) extern "C" fn test_free_string(
    _ctx: *mut raw::RedisModuleCtx,
    s: *mut raw::RedisModuleString,
) {
    if s.is_null() {
        return;
    }

    let test_string = as_test_string_mut(s);
    // RedisModuleString is reference counted; only free the boxed shim when the
    // last ValkeyString wrapper releases it.
    test_string.ref_count -= 1;
    let should_free = test_string.ref_count == 0;

    if should_free {
        let raw_test_string = s.cast::<TestValkeyString>();
        // Recreate the Box only at the final release so Rust can drop the Vec
        // allocation exactly once.
        unsafe {
            drop(Box::from_raw(raw_test_string));
        };
    }
}

pub(super) extern "C" fn test_string_ptr_len(
    s: *const raw::RedisModuleString,
    len: *mut usize,
) -> *const c_char {
    let test_string = as_test_string(s);
    if !len.is_null() {
        let data_len = test_string.data.len();
        unsafe {
            *len = data_len;
        }
    }
    // The returned pointer stays valid while the shim object remains alive.
    let data_ptr = test_string.data.as_ptr();
    data_ptr.cast::<c_char>()
}

pub(super) extern "C" fn test_retain_string(
    _ctx: *mut raw::RedisModuleCtx,
    s: *mut raw::RedisModuleString,
) {
    if !s.is_null() {
        // ValkeyString::new and create_and_retain rely on RetainString keeping
        // the underlying RedisModuleString alive past the current wrapper.
        let test_string = as_test_string_mut(s);
        test_string.ref_count += 1;
    }
}

pub(super) extern "C" fn test_string_append_buffer(
    _ctx: *mut raw::RedisModuleCtx,
    s: *mut raw::RedisModuleString,
    buf: *const c_char,
    len: usize,
) -> c_int {
    // RedisModule_StringAppendBuffer mutates the string in place.
    let data = buffer_as_slice(buf, len);
    let test_string = as_test_string_mut(s);
    test_string.data.extend_from_slice(data);

    raw::REDISMODULE_OK as c_int
}

pub(super) extern "C" fn test_string_compare(
    a: *const raw::RedisModuleString,
    b: *const raw::RedisModuleString,
) -> c_int {
    // The raw API returns an integer ordering, not Rust's Ordering enum.
    let a = as_test_string(a);
    let b = as_test_string(b);
    let ordering = a.data.cmp(&b.data);

    match ordering {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub(super) extern "C" fn test_string_to_longlong(
    s: *const raw::RedisModuleString,
    ll: *mut c_longlong,
) -> c_int {
    // Match RedisModule_StringToLongLong closely enough for command unit tests:
    // valid decimal UTF-8 fills the output pointer and invalid input returns ERR.
    let test_string = as_test_string(s);
    let data = test_string.data.as_slice();

    let string = match std::str::from_utf8(data) {
        Ok(string) => string,
        Err(_) => return raw::REDISMODULE_ERR as c_int,
    };

    let value = match string.parse::<c_longlong>() {
        Ok(value) => value,
        Err(_) => return raw::REDISMODULE_ERR as c_int,
    };

    if !ll.is_null() {
        // Redis writes parsed output through the caller-provided pointer.
        unsafe {
            *ll = value;
        }
    }
    raw::REDISMODULE_OK as c_int
}

#[cfg(test)]
mod tests {
    use super::super::setup_test_shims;
    use crate::{Context, ValkeyString};

    #[test]
    fn create_string_returns_readable_valkey_string() {
        setup_test_shims();

        let value = ValkeyString::create(None, "hello");

        assert_eq!(value.as_slice(), b"hello");
        assert_eq!(value.len(), 5);
    }

    #[test]
    fn create_string_supports_context_helper() {
        let ctx = Context::test(None);

        let value = ctx.create_string("hello");

        assert_eq!(value.as_slice(), b"hello");
    }

    #[test]
    fn create_string_supports_clone_and_append() {
        setup_test_shims();
        let mut value = ValkeyString::create(None, "hel");

        value.append("lo");
        let clone = value.clone();

        assert_eq!(value.as_slice(), b"hello");
        assert_eq!(clone.as_slice(), b"hello");
        assert_eq!(value, clone);
    }
}
