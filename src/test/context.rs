use crate::{raw, Context};
use std::collections::HashMap;
use std::ffi::c_ulonglong;
use std::ops::Deref;

pub struct TestContext {
    context: Context,
    // Keeps the fake raw context pointer valid for this TestContext.
    #[allow(dead_code)]
    data: Box<HashMap<String, String>>,
}

impl TestContext {
    #[must_use]
    pub fn new(data: HashMap<String, String>) -> Self {
        super::setup_test_shims();
        let mut data = Box::new(data);
        let ctx = data.as_mut() as *mut HashMap<String, String> as *mut raw::RedisModuleCtx;
        Self {
            context: Context::new(ctx),
            data,
        }
    }
}

impl Deref for TestContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl Context {
    #[must_use]
    pub fn test(data: HashMap<String, String>) -> TestContext {
        TestContext::new(data)
    }
}

pub(super) extern "C" fn test_get_client_id(ctx: *mut raw::RedisModuleCtx) -> c_ulonglong {
    let data = unsafe { &*(ctx as *const HashMap<String, String>) };
    data.get("client_id")
        .and_then(|client_id| client_id.parse().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_client_id() {
        let data = HashMap::from([("client_id".to_string(), "10".to_string())]);
        let ctx = Context::test(data);
        assert_eq!(ctx.get_client_id(), 10);
    }
}
