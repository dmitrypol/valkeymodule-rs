use crate::{raw, ValkeyString};
use libc::{c_char, c_ulonglong, size_t};
use std::ptr::null_mut;
use std::str::FromStr;

impl ValkeyString {
    pub fn test<T: Into<Vec<u8>>>(data: T) -> ValkeyString {
        super::setup_test_shims();
        let data = Box::into_raw(Box::new(data.into()));
        let inner = data.cast::<raw::RedisModuleString>();
        ValkeyString::from_redis_module_string(null_mut(), inner)
    }
}

pub(super) extern "C" fn test_string_ptr_len(
    string: *const raw::RedisModuleString,
    len: *mut size_t,
) -> *const c_char {
    let data = unsafe { &*(string as *const Vec<u8>) };
    unsafe {
        *len = data.len();
    }
    data.as_ptr().cast::<c_char>()
}

pub(super) extern "C" fn test_free_string(
    _ctx: *mut raw::RedisModuleCtx,
    _string: *mut raw::RedisModuleString,
) {
}

pub(super) extern "C" fn test_retain_string(
    _ctx: *mut raw::RedisModuleCtx,
    _string: *mut raw::RedisModuleString,
) {
}

pub(super) extern "C" fn test_string_to_longlong(
    string: *const raw::RedisModuleString,
    value: *mut i64,
) -> libc::c_int {
    test_parse_string(string, value)
}

pub(super) extern "C" fn test_string_to_ulonglong(
    string: *const raw::RedisModuleString,
    value: *mut c_ulonglong,
) -> libc::c_int {
    test_parse_string(string, value)
}

pub(super) extern "C" fn test_string_to_double(
    string: *const raw::RedisModuleString,
    value: *mut f64,
) -> libc::c_int {
    test_parse_string(string, value)
}

fn test_parse_string<T: FromStr>(
    string: *const raw::RedisModuleString,
    value: *mut T,
) -> libc::c_int {
    // ValkeyString::test stores bytes as Box<Vec<u8>> behind the opaque
    // RedisModuleString pointer, so test shims cast it back before parsing.
    let data = unsafe { &*(string as *const Vec<u8>) };
    let Ok(data) = std::str::from_utf8(data) else {
        return raw::Status::Err as libc::c_int;
    };
    let Ok(data) = data.parse::<T>() else {
        return raw::Status::Err as libc::c_int;
    };
    // Redis numeric conversion APIs report the parsed value through an out pointer.
    unsafe {
        *value = data;
    }
    raw::Status::Ok as libc::c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valkey_string_to_longlong() {
        let string = ValkeyString::test("-42");
        let mut value = 0;

        let status =
            unsafe { raw::RedisModule_StringToLongLong.unwrap()(string.inner, &mut value) };

        assert_eq!(raw::Status::from(status), raw::Status::Ok);
        assert_eq!(value, -42);
    }

    #[test]
    fn test_valkey_string_to_ulonglong() {
        let string = ValkeyString::test("42");
        let mut value: c_ulonglong = 0;

        let status =
            unsafe { raw::RedisModule_StringToULongLong.unwrap()(string.inner, &mut value) };

        assert_eq!(raw::Status::from(status), raw::Status::Ok);
        assert_eq!(value, 42);
    }
    #[test]
    fn test_valkey_string_parse_float() {
        let string = ValkeyString::test("42.5");

        assert_eq!(string.parse_float().unwrap(), 42.5);
    }
}
