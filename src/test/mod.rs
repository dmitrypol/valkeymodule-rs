mod context;
mod valkey_string;

use crate::raw;
use std::sync::Once;

static INIT: Once = Once::new();
fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        raw::RedisModule_GetClientId = Some(context::test_get_client_id);
        raw::RedisModule_GetClientNameById = Some(context::test_get_client_name_by_id);
        raw::RedisModule_SetClientNameById = Some(context::test_set_client_name_by_id);
        raw::RedisModule_GetClientUserNameById = Some(context::test_get_client_username_by_id);
        raw::RedisModule_GetClientCertificate = Some(context::test_get_client_certificate);
        raw::RedisModule_DeauthenticateAndCloseClient =
            Some(context::test_deauthenticate_and_close_client);
        raw::RedisModule_StringPtrLen = Some(valkey_string::test_string_ptr_len);
        raw::RedisModule_FreeString = Some(valkey_string::test_free_string);
        raw::RedisModule_RetainString = Some(valkey_string::test_retain_string);
        raw::RedisModule_StringToLongLong = Some(valkey_string::test_string_to_longlong);
    })
}
