use std::sync::Once;

mod context;
mod valkey_string;

use crate::raw;
pub use context::TestContext;
pub use valkey_string::TestValkeyString;

static INIT: Once = Once::new();

fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        // Context calls C API, this routes to shim instead of live Valkey
        raw::RedisModule_GetClientId = Some(context::test_get_client_id);
        raw::RedisModule_StringPtrLen = Some(valkey_string::test_string_ptr_len);
        raw::RedisModule_FreeString = Some(valkey_string::test_free_string);
        raw::RedisModule_RetainString = Some(valkey_string::test_retain_string);
    })
}
