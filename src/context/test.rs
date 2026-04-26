use std::ops::Deref;
use std::os::raw::c_ulonglong;
use std::ptr;
use std::sync::Once;

use super::Context;
use crate::{raw, ValkeyString};

const TEST_CLIENT_ID: u64 = 1;
const TEST_CLIENT_NAME: &str = "test-client";

static INIT: Once = Once::new();

extern "C" fn test_get_client_id(_ctx: *mut raw::RedisModuleCtx) -> c_ulonglong {
    TEST_CLIENT_ID as c_ulonglong
}

extern "C" fn test_get_client_name_by_id(
    _ctx: *mut raw::RedisModuleCtx,
    client_id: c_ulonglong,
) -> *mut raw::RedisModuleString {
    if client_id == TEST_CLIENT_ID as c_ulonglong {
        ValkeyString::create(None, TEST_CLIENT_NAME).take()
    } else {
        ptr::null_mut()
    }
}

#[cfg(any(test, feature = "test-mocks"))]
fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        crate::redismodule_test::setup_test_shims();

        let get_client_id = raw::RedisModule_GetClientId;
        if get_client_id.is_none() {
            raw::RedisModule_GetClientId = Some(test_get_client_id);
            raw::RedisModule_GetClientNameById = Some(test_get_client_name_by_id);
        }
    });
}

/// Owned context for tests and crates built with the `test-mocks` feature.
///
/// It dereferences to [`Context`], so command handlers that accept `&Context`
/// can receive `&Context::test()` in tests without a running Valkey server.
#[derive(Debug)]
pub struct TestContext {
    context: Context,
    _allocation: Box<u8>,
}

impl TestContext {
    pub fn new() -> Self {
        setup_test_shims();

        let mut allocation = Box::new(0_u8);
        let ctx = (&mut *allocation as *mut u8).cast();

        Self {
            context: Context::new(ctx),
            _allocation: allocation,
        }
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for TestContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl Context {
    #[cfg(any(test, feature = "test-mocks"))]
    #[must_use]
    pub fn test() -> TestContext {
        TestContext::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_uses_non_null_test_allocation() {
        let ctx = Context::test();

        assert!(!ctx.ctx.is_null());

        fn accepts_context(_: &Context) {}

        accepts_context(&ctx);
    }

    #[test]
    fn test_get_client_id() {
        let ctx = Context::test();

        assert_eq!(ctx.get_client_id(), TEST_CLIENT_ID);
    }

    #[test]
    fn test_get_client_name() {
        let ctx = Context::test();
        let client_name = ctx.get_client_name().unwrap();

        assert_eq!(client_name.try_as_str().unwrap(), TEST_CLIENT_NAME);
        assert!(ctx.get_client_name_by_id(TEST_CLIENT_ID).is_ok());
        assert!(ctx.get_client_name_by_id(0).is_err());
    }
}
