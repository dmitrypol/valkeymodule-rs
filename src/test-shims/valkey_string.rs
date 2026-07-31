use crate::{raw, ValkeyString};
use std::os::raw::c_char;
use std::ptr::null_mut;
use std::sync::Arc;

impl ValkeyString {
    pub fn test<T: Into<String>>(data: T) -> ValkeyString {
        super::setup_test_shims();
        let data = Arc::into_raw(Arc::new(data.into()));
        let inner = data.cast_mut().cast::<raw::RedisModuleString>();
        ValkeyString::from_redis_module_string(null_mut(), inner)
    }
}

pub(super) extern "C" fn string_ptr_len(
    string: *const raw::RedisModuleString,
    len: *mut usize,
) -> *const c_char {
    let data = unsafe { &*string.cast::<String>() };
    unsafe {
        *len = data.len();
    }
    data.as_ptr().cast::<c_char>()
}

pub(super) extern "C" fn free_string(
    _ctx: *mut raw::RedisModuleCtx,
    string: *mut raw::RedisModuleString,
) {
    if string.is_null() {
        return;
    }

    unsafe {
        Arc::decrement_strong_count(string.cast::<String>());
    }
}

pub(super) extern "C" fn retain_string(
    _ctx: *mut raw::RedisModuleCtx,
    string: *mut raw::RedisModuleString,
) {
    if string.is_null() {
        return;
    }

    unsafe {
        Arc::increment_strong_count(string.cast::<String>());
    }
}
