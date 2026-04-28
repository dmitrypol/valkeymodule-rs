use crate::{raw, Context, RedisModuleClientInfo, RedisModuleCtx, ValkeyString};
use libc::c_ulonglong;
use std::ops::Deref;
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::ptr::NonNull;

// Keep the fake client ID stable so tests can make exact assertions without a
// running Valkey server.
const TEST_CLIENT_ID: c_ulonglong = 1;
// Keep the fake client name paired with TEST_CLIENT_ID so example command tests
// can exercise Context::get_client_name without a real connection.
const TEST_CLIENT_NAME: &str = "test-client-name";
// Keep the fake username paired with TEST_CLIENT_ID so example command tests
// can exercise Context::get_client_username without a real connection.
const TEST_CLIENT_USERNAME: &str = "test-client-username";
// Keep the fake client certificate paired with TEST_CLIENT_ID so unit tests can
// exercise Context::get_client_cert without a TLS-enabled Valkey connection.
const TEST_CLIENT_CERT: &str = "test-client-cert";

// Shim for RedisModule_GetClientId used by Context::test().
pub(super) extern "C" fn test_get_client_id(_ctx: *mut raw::RedisModuleCtx) -> c_ulonglong {
    TEST_CLIENT_ID
}

pub(super) extern "C" fn test_get_client_name_by_id(
    _ctx: *mut RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    // Match RedisModule_GetClientNameById behavior for an unknown client: a
    // null pointer makes Context convert the result into ValkeyError.
    if client_id != TEST_CLIENT_ID {
        return ptr::null_mut();
    }

    // Return ownership of the raw RedisModuleString pointer to the caller, just
    // like the raw API does. The ValkeyString shim still owns the allocation.
    ValkeyString::create(None, TEST_CLIENT_NAME).take()
}

pub(super) extern "C" fn test_get_client_username_by_id(
    _ctx: *mut RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    // Match RedisModule_GetClientUserNameById behavior for an unknown client:
    // a null pointer makes Context convert the result into ValkeyError.
    if client_id != TEST_CLIENT_ID {
        return ptr::null_mut();
    }
    // Return ownership of the raw RedisModuleString pointer to the caller, just
    // like the raw API does. The ValkeyString shim still owns the allocation.
    ValkeyString::create(None, TEST_CLIENT_USERNAME).take()
}

pub(super) extern "C" fn test_get_client_certificate(
    _ctx: *mut RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    // Match RedisModule_GetClientCertificate behavior for a client without an
    // available certificate: a null pointer makes Context return ValkeyError.
    if client_id != TEST_CLIENT_ID {
        return ptr::null_mut();
    }
    // Return ownership of the raw RedisModuleString pointer to the caller, just
    // like the raw API does. The ValkeyString shim still owns the allocation.
    ValkeyString::create(None, TEST_CLIENT_CERT).take()
}

pub(super) extern "C" fn test_get_client_info_by_id(ci: *mut c_void, client_id: u64) -> c_int {
    // Match RedisModule_GetClientInfoById for invalid input: return ERR and
    // leave the caller-provided output struct untouched.
    if ci.is_null() || client_id != TEST_CLIENT_ID {
        return raw::REDISMODULE_ERR as c_int;
    }

    // The production API fills the caller-owned RedisModuleClientInfo struct in
    // place, so cast the opaque pointer back to the expected test layout.
    let client_info = unsafe { &mut *ci.cast::<RedisModuleClientInfo>() };
    client_info.version = 1;
    client_info.flags = 0;
    client_info.id = client_id;
    client_info.addr = [0; 46];
    client_info.port = 6379;
    client_info.db = 0;

    // Keep the address null-terminated because Context::get_client_ip reads it
    // through CStr, just like it does against the real Valkey API.
    let addr = b"127.0.0.1\0";
    for (dest, src) in client_info.addr.iter_mut().zip(addr.iter()) {
        *dest = *src as _;
    }

    raw::REDISMODULE_OK as c_int
}

pub(super) extern "C" fn test_set_client_name_by_id(
    client_id: u64,
    name: *mut raw::RedisModuleString,
) -> c_int {
    if client_id == TEST_CLIENT_ID && !name.is_null() {
        raw::REDISMODULE_OK as c_int
    } else {
        raw::REDISMODULE_ERR as c_int
    }
}

pub(super) extern "C" fn test_deauthenticate_and_close_client(
    _ctx: *mut RedisModuleCtx,
    client_id: u64,
) -> c_int {
    if client_id == TEST_CLIENT_ID {
        raw::REDISMODULE_OK as c_int
    } else {
        raw::REDISMODULE_ERR as c_int
    }
}

#[derive(Debug)]
pub struct TestContext {
    // The production Context type is still exercised; only the raw Valkey API
    // functions it calls are replaced with local shims.
    inner: Context,
}

impl TestContext {
    /// Create a test context backed by local raw API shims.
    ///
    /// This is suitable for unit tests that exercise `Context`-based command
    /// helpers without running inside a Valkey server.
    #[must_use]
    pub fn new() -> Self {
        super::setup_test_shims();
        // RedisModuleCtx is opaque to Rust. The shimmed functions above do not
        // dereference it, but Context should still carry a non-null handle.
        let ctx = NonNull::<raw::RedisModuleCtx>::dangling().as_ptr();
        Self {
            inner: Context::new(ctx),
        }
    }

    /// Borrow the wrapped `Context`.
    #[must_use]
    pub fn inner(&self) -> &Context {
        &self.inner
    }
}

impl Default for TestContext {
    fn default() -> Self {
        // Keep Default equivalent to new() so tests can construct a context via
        // standard helper patterns without bypassing shim installation.
        Self::new()
    }
}

impl AsRef<Context> for TestContext {
    fn as_ref(&self) -> &Context {
        // AsRef supports helper functions that accept generic context-like
        // values while still exercising the real Context implementation.
        self.inner()
    }
}

impl Deref for TestContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        // Let tests pass TestContext anywhere an immutable Context is expected.
        self.inner()
    }
}

impl Context {
    /// Create a [`TestContext`] with the raw API shims needed by unit tests.
    #[must_use]
    #[cfg(any(test, feature = "unit-tests"))]
    pub fn test() -> TestContext {
        // Expose test construction from Context so example commands can use the
        // same call shape as production command handlers: `&Context`.
        TestContext::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_uses_non_null_dummy_handle() {
        let test = Context::test();
        assert!(!test.inner().ctx.is_null())
    }

    #[test]
    fn test_context_derefs_to_context() {
        fn assert_context(ctx: &Context) {
            assert!(!ctx.ctx.is_null());
        }
        assert_context(&Context::test());
    }

    #[test]
    fn test_get_client_id() {
        let test = Context::test();
        assert_eq!(test.get_client_id(), TEST_CLIENT_ID)
    }

    #[test]
    fn test_get_client_name() {
        let test = Context::test();
        assert_eq!(
            test.get_client_name().unwrap(),
            test.create_string(TEST_CLIENT_NAME)
        )
    }

    #[test]
    fn test_get_client_username() {
        let test = Context::test();
        assert_eq!(
            test.get_client_username().unwrap(),
            test.create_string(TEST_CLIENT_USERNAME)
        )
    }

    #[test]
    fn test_set_client_name() {
        let test = Context::test();
        let client_name = test.create_string("new-client-name");
        assert_eq!(test.set_client_name(&client_name), raw::Status::Ok)
    }

    #[test]
    fn test_get_client_cert() {
        let test = Context::test();
        assert_eq!(
            test.get_client_cert().unwrap(),
            test.create_string(TEST_CLIENT_CERT)
        )
    }

    #[test]
    fn test_get_client_info() {
        let test = Context::test();
        let client_info = test.get_client_info().unwrap();
        assert_eq!(client_info.version, 1);
        assert_eq!(client_info.id, TEST_CLIENT_ID);
    }

    #[test]
    fn test_deauthenticate_and_close_client() {
        let test = Context::test();
        assert_eq!(test.deauthenticate_and_close_client(), raw::Status::Ok)
    }
}
