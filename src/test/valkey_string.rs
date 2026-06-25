use crate::{raw, ValkeyString};
use libc::{c_char, size_t};
use std::ptr::null_mut;

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
    let data = unsafe { &*(string as *const Vec<u8>) };
    let Ok(data) = std::str::from_utf8(data) else {
        return raw::Status::Err as libc::c_int;
    };
    let Ok(data) = data.parse::<i64>() else {
        return raw::Status::Err as libc::c_int;
    };
    unsafe {
        *value = data;
    }
    raw::Status::Ok as libc::c_int
}
