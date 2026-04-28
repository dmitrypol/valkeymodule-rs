//! Unit test helpers for code that needs a [`crate::Context`] without running
//! inside a Valkey server.

mod context;
mod valkey_string;

use crate::raw;
use crate::test::context::*;
use crate::test::valkey_string::*;
use std::sync::Once;

static INIT: Once = Once::new();

// Unit tests run outside Valkey, so install only the raw API functions that the
// tested Context methods need.
fn setup_test_shims() {
    INIT.call_once(|| unsafe {
        let get_client_id = raw::RedisModule_GetClientId;
        if get_client_id.is_none() {
            raw::RedisModule_GetClientId = Some(test_get_client_id);
            raw::RedisModule_GetClientNameById = Some(test_get_client_name_by_id);
            raw::RedisModule_GetClientUserNameById = Some(test_get_client_username_by_id);
            raw::RedisModule_GetClientCertificate = Some(test_get_client_certificate);
            raw::RedisModule_GetClientInfoById = Some(test_get_client_info_by_id);
            raw::RedisModule_SetClientNameById = Some(test_set_client_name_by_id);
            raw::RedisModule_DeauthenticateAndCloseClient =
                Some(test_deauthenticate_and_close_client);
            raw::RedisModule_CreateString = Some(test_create_string);
            raw::RedisModule_CreateStringFromString = Some(test_create_string_from_string);
            raw::RedisModule_FreeString = Some(test_free_string);
            raw::RedisModule_StringPtrLen = Some(test_string_ptr_len);
            raw::RedisModule_RetainString = Some(test_retain_string);
            raw::RedisModule_StringAppendBuffer = Some(test_string_append_buffer);
            raw::RedisModule_StringCompare = Some(test_string_compare);
            raw::RedisModule_StringToLongLong = Some(test_string_to_longlong);
        }
    })
}
