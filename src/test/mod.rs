//! Unit test helpers for code that needs a [`crate::Context`] without running
//! inside a Valkey server.

mod context;

use crate::raw;
use std::sync::Once;

static INIT: Once = Once::new();

// Unit tests run outside Valkey, so install only the raw API functions that the
// tested Context methods need.
fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        let get_client_id = raw::RedisModule_GetClientId;
        if get_client_id.is_none() {
            raw::RedisModule_GetClientId = Some(context::test_get_client_id);
        }
    })
}
