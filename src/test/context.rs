use crate::{raw, Context};
use libc::c_ulonglong;
use std::ops::Deref;
use std::ptr::NonNull;

// Keep the fake client ID stable so tests can make exact assertions without a
// running Valkey server.
const TEST_CLIENT_ID: c_ulonglong = 1;

// Shim for RedisModule_GetClientId used by Context::test().
pub(super) extern "C" fn test_get_client_id(_ctx: *mut raw::RedisModuleCtx) -> c_ulonglong {
    TEST_CLIENT_ID
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
        Self::new()
    }
}

impl AsRef<Context> for TestContext {
    fn as_ref(&self) -> &Context {
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
}
