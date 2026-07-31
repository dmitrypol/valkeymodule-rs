mod valkey_string;

use crate::raw;
use std::sync::Once;

static INIT: Once = Once::new();

fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        raw::RedisModule_StringPtrLen = Some(valkey_string::string_ptr_len);
        raw::RedisModule_FreeString = Some(valkey_string::free_string);
        raw::RedisModule_RetainString = Some(valkey_string::retain_string);
    });
}
