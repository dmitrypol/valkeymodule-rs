use crate::{raw, ValkeyString};
use std::ops::Deref;
use std::os::raw::c_char;

pub struct TestValkeyString {
    valkey_string: ValkeyString,
    // Keeps the fake raw RedisModuleString pointer valid for this TestValkeyString.
    #[allow(dead_code)]
    data: Box<Vec<u8>>,
}

impl TestValkeyString {
    #[must_use]
    pub fn new<T: Into<Vec<u8>>>(data: T) -> Self {
        super::setup_test_shims();
        let mut data = Box::new(data.into());
        let inner = data.as_mut() as *mut Vec<u8> as *mut raw::RedisModuleString;
        Self {
            valkey_string: ValkeyString::from_redis_module_string(std::ptr::null_mut(), inner),
            data,
        }
    }
}

impl Deref for TestValkeyString {
    type Target = ValkeyString;

    fn deref(&self) -> &Self::Target {
        &self.valkey_string
    }
}

impl ValkeyString {
    #[must_use]
    pub fn test<T: Into<Vec<u8>>>(data: T) -> TestValkeyString {
        TestValkeyString::new(data)
    }
}

pub(super) extern "C" fn test_string_ptr_len(
    s: *const raw::RedisModuleString,
    len: *mut usize,
) -> *const c_char {
    let data = unsafe { &*(s as *const Vec<u8>) };
    if !len.is_null() {
        unsafe { *len = data.len() };
    }
    data.as_ptr().cast::<c_char>()
}

pub(super) extern "C" fn test_free_string(
    _ctx: *mut raw::RedisModuleCtx,
    _s: *mut raw::RedisModuleString,
) {
    // No-op: TestValkeyString owns the backing Vec<u8>.
}

pub(super) extern "C" fn test_retain_string(
    _ctx: *mut raw::RedisModuleCtx,
    _s: *mut raw::RedisModuleString,
) {
    // No-op: TestValkeyString owns the backing Vec<u8>, so refcounting is moot.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_slice() {
        let vs = ValkeyString::test("hello");
        assert_eq!(vs.as_slice(), b"hello");
        assert_eq!(vs.len(), 5);
        assert!(!vs.is_empty());
        assert_eq!(vs.try_as_str().unwrap(), "hello");
    }

    #[test]
    fn test_empty() {
        let vs = ValkeyString::test("");
        assert_eq!(vs.len(), 0);
        assert!(vs.is_empty());
    }

    #[test]
    fn test_non_utf8() {
        let vs = ValkeyString::test(vec![0xff, 0xfe, 0xfd]);
        assert_eq!(vs.as_slice(), &[0xff, 0xfe, 0xfd]);
        assert!(vs.try_as_str().is_err());
    }
}
