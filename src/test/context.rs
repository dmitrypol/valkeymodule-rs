use crate::{raw, Context};
use std::collections::HashMap;
use std::ptr::null_mut;

impl Context {
    pub fn test(data: HashMap<String, String>) -> Self {
        super::setup_test_shims();
        // Tests do not have a real RedisModuleCtx. Store fixture data behind a
        // raw pointer and let the test shims cast it back when Valkey APIs read it.
        let data = Box::into_raw(Box::new(data));
        let ctx = data.cast::<raw::RedisModuleCtx>();
        Context::new(ctx)
    }
}

fn test_context_data<'a>(ctx: *mut raw::RedisModuleCtx) -> &'a HashMap<String, String> {
    // Context::test stores test context data as Box<HashMap<_, _>> and passes
    // the allocation through the RedisModuleCtx pointer type. This helper is
    // only valid for contexts produced by Context::test.
    unsafe { &*ctx.cast::<HashMap<String, String>>() }
}

// Look up a string fixture by key and return it as a fake RedisModuleString.
// Missing keys mirror Valkey APIs that return null for unavailable client data.
fn test_context_string(ctx: *mut raw::RedisModuleCtx, key: &str) -> *mut raw::RedisModuleString {
    let data = test_context_data(ctx);
    data.get(key)
        .map(|value| {
            let tmp = Box::into_raw(Box::new(value.as_bytes().to_vec()));
            tmp.cast::<raw::RedisModuleString>()
        })
        .unwrap_or(null_mut())
}

pub(super) extern "C" fn test_get_client_id(ctx: *mut raw::RedisModuleCtx) -> u64 {
    let data = test_context_data(ctx);
    data.get("client_id")
        .and_then(|client_id| client_id.parse().ok())
        .unwrap_or_default()
}

pub(super) extern "C" fn test_get_client_name_by_id(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    if client_id != test_get_client_id(ctx) {
        return null_mut();
    }
    test_context_string(ctx, "client_name")
}

pub(super) extern "C" fn test_set_client_name_by_id(
    client_id: u64,
    _client_name: *mut raw::RedisModuleString,
) -> libc::c_int {
    if client_id == 0 {
        raw::Status::Err as libc::c_int
    } else {
        raw::Status::Ok as libc::c_int
    }
}

pub(super) extern "C" fn test_get_client_username_by_id(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    if client_id != test_get_client_id(ctx) {
        return null_mut();
    }
    test_context_string(ctx, "client_username")
}

pub(super) extern "C" fn test_get_client_certificate(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    if client_id != test_get_client_id(ctx) {
        return null_mut();
    }
    test_context_string(ctx, "client_cert")
}

pub(super) extern "C" fn test_deauthenticate_and_close_client(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> libc::c_int {
    if client_id == test_get_client_id(ctx) {
        raw::Status::Ok as libc::c_int
    } else {
        raw::Status::Err as libc::c_int
    }
}
