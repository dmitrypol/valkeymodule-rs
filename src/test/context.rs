use crate::{raw, Context, RedisModuleClientInfo, RedisModuleCtx, ValkeyString};
use libc::c_ulonglong;
use std::collections::HashMap;
use std::ops::Deref;
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const TEST_CLIENT_NAME: &str = "test-client-name";
const TEST_CLIENT_USERNAME: &str = "test-client-username";
const TEST_CLIENT_CERT: &str = "test-client-cert";

static NEXT_TEST_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);
static TEST_CONTEXT_DATA: OnceLock<Mutex<HashMap<usize, HashMap<String, String>>>> =
    OnceLock::new();

fn test_context_data() -> &'static Mutex<HashMap<usize, HashMap<String, String>>> {
    TEST_CONTEXT_DATA.get_or_init(|| Mutex::new(HashMap::new()))
}

fn test_context_ptr() -> *mut raw::RedisModuleCtx {
    // NonNull::<raw::RedisModuleCtx>::dangling().as_ptr()
    NEXT_TEST_CONTEXT_ID.fetch_add(1, Ordering::Relaxed) as *mut raw::RedisModuleCtx
}

fn register_test_context_data(ctx: *mut raw::RedisModuleCtx, data: HashMap<String, String>) {
    test_context_data()
        .lock()
        .expect("test context data lock poisoned")
        .insert(ctx as usize, data);
}

fn unregister_test_context_data(ctx: *mut raw::RedisModuleCtx) {
    test_context_data()
        .lock()
        .expect("test context data lock poisoned")
        .remove(&(ctx as usize));
}

// Shim for RedisModule_GetClientId used by Context::test.
pub(super) extern "C" fn test_get_client_id(ctx: *mut raw::RedisModuleCtx) -> c_ulonglong {
    test_context_data()
        .lock()
        .expect("test context data lock poisoned")
        .get(&(ctx as usize))
        .and_then(|data| data.get("client_id"))
        .and_then(|client_id| client_id.parse().ok())
        .unwrap_or(0)
}

pub(super) extern "C" fn test_get_client_name_by_id(
    _ctx: *mut RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    // Match RedisModule_GetClientNameById behavior for an unknown client: a
    // null pointer makes Context convert the result into ValkeyError.
    if client_id != 0 {
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
    if client_id != 0 {
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
    if client_id != 0 {
        return ptr::null_mut();
    }
    // Return ownership of the raw RedisModuleString pointer to the caller, just
    // like the raw API does. The ValkeyString shim still owns the allocation.
    ValkeyString::create(None, TEST_CLIENT_CERT).take()
}

pub(super) extern "C" fn test_get_client_info_by_id(ci: *mut c_void, client_id: u64) -> c_int {
    // Match RedisModule_GetClientInfoById for invalid input: return ERR and
    // leave the caller-provided output struct untouched.
    if ci.is_null() || client_id != 0 {
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
    if client_id == 0 && !name.is_null() {
        raw::REDISMODULE_OK as c_int
    } else {
        raw::REDISMODULE_ERR as c_int
    }
}

pub(super) extern "C" fn test_deauthenticate_and_close_client(
    ctx: *mut RedisModuleCtx,
    client_id: u64,
) -> c_int {
    if client_id == test_get_client_id(ctx) {
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
    pub data: HashMap<String, String>,
}

impl TestContext {
    /// Create a test context backed by local raw API shims.
    ///
    /// This is suitable for unit tests that exercise `Context`-based command
    /// helpers without running inside a Valkey server.
    #[must_use]
    pub fn new(data: Option<HashMap<String, String>>) -> Self {
        super::setup_test_shims();
        // RedisModuleCtx is opaque to Rust. The shimmed functions above do not
        // dereference it, but Context should still carry a non-null handle.
        let ctx = test_context_ptr();
        let data = data.unwrap_or_default();
        register_test_context_data(ctx, data.clone());
        Self {
            inner: Context::new(ctx),
            data,
        }
    }

    /// Borrow the wrapped `Context`.
    #[must_use]
    pub fn inner(&self) -> &Context {
        &self.inner
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        unregister_test_context_data(self.inner.ctx);
    }
}

impl Default for TestContext {
    fn default() -> Self {
        // Keep Default equivalent to new(None) so tests can construct a context
        // via standard helper patterns without bypassing shim installation.
        Self::new(None)
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
    pub fn test(data: Option<HashMap<String, String>>) -> TestContext {
        // Expose test construction from Context so example commands can use the
        // same call shape as production command handlers: `&Context`.
        TestContext::new(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_uses_non_null_dummy_handle() {
        let test = TestContext::default();
        assert!(!test.inner().ctx.is_null())
    }

    #[test]
    fn test_context_stores_data() {
        let test = Context::test(Some(HashMap::from([(
            "key".to_string(),
            "value".to_string(),
        )])));
        assert_eq!(test.data.get("key"), Some(&"value".to_string()))
    }
    #[test]
    fn test_context_derefs_to_context() {
        fn assert_context(ctx: &Context) {
            assert!(!ctx.ctx.is_null());
        }
        assert_context(&TestContext::default());
    }

    #[test]
    fn test_get_client_id() {
        let data = HashMap::from([("client_id".to_string(), "10".to_string())]);
        let test = Context::test(Some(data));
        assert_eq!(test.get_client_id(), 10)
    }

    #[test]
    fn test_get_client_name() {
        let test = Context::test(None);
        assert_eq!(
            test.get_client_name().unwrap(),
            test.create_string(TEST_CLIENT_NAME)
        )
    }

    #[test]
    fn test_get_client_username() {
        let test = Context::test(None);
        assert_eq!(
            test.get_client_username().unwrap(),
            test.create_string(TEST_CLIENT_USERNAME)
        )
    }

    #[test]
    fn test_set_client_name() {
        let test = Context::test(None);
        let client_name = test.create_string("new-client-name");
        assert_eq!(test.set_client_name(&client_name), raw::Status::Ok)
    }

    #[test]
    fn test_get_client_cert() {
        let test = Context::test(None);
        assert_eq!(
            test.get_client_cert().unwrap(),
            test.create_string(TEST_CLIENT_CERT)
        )
    }

    #[test]
    fn test_get_client_info() {
        let test = Context::test(None);
        let client_info = test.get_client_info().unwrap();
        assert_eq!(client_info.version, 1);
        assert_eq!(client_info.id, 0);
    }

    #[test]
    fn test_deauthenticate_and_close_client() {
        let test = Context::test(None);
        assert_eq!(test.deauthenticate_and_close_client(), raw::Status::Ok)
    }
}
