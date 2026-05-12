use std::sync::Once;

mod context;

use crate::raw;
pub use context::TestContext;

static INIT: Once = Once::new();

fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        // Context calls C API, this routes to shim instead of live Valkey
        raw::RedisModule_GetClientId = Some(context::test_get_client_id);
    })
}
