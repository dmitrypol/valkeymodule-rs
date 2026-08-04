use anyhow::{Context, Result};
use redis::RedisError;
use valkey_module::ValkeyValue;

use crate::utils::{get_valkey_connection, start_valkey_server_with_module};
use crate::{FAILED_TO_CONNECT_TO_SERVER, FAILED_TO_START_SERVER};

#[test]
fn test_helper_version() -> Result<()> {
    let port: u16 = 6481;
    let _guards = vec![start_valkey_server_with_module("test_helper", port)
        .with_context(|| FAILED_TO_START_SERVER)?];
    let mut con = get_valkey_connection(port).with_context(|| FAILED_TO_CONNECT_TO_SERVER)?;

    let res: Vec<i64> = redis::cmd("test_helper.version")
        .query(&mut con)
        .with_context(|| "failed to run test_helper.version")?;
    assert!(res[0] > 0);

    let res3: String = redis::cmd("test_helper.name")
        .query(&mut con)
        .with_context(|| "failed to run test_helper.name")?;
    assert_eq!(res3, "test_helper.name");

    Ok(())
}

#[test]
fn test_command_name() -> Result<()> {
    let port: u16 = 6482;
    let _guards = vec![start_valkey_server_with_module("test_helper", port)
        .with_context(|| FAILED_TO_START_SERVER)?];
    let mut con = get_valkey_connection(port).with_context(|| FAILED_TO_CONNECT_TO_SERVER)?;

    let res: Result<String, RedisError> = redis::cmd("test_helper.name").query(&mut con);
    let info: String = redis::cmd("info")
        .arg(&["server"])
        .query(&mut con)
        .with_context(|| "failed to run test_helper.name")?;

    if let Ok(ver) = valkey_module::Context::version_from_info(ValkeyValue::SimpleString(info)) {
        if ver.major > 6
            || (ver.major == 6 && ver.minor > 2)
            || (ver.major == 6 && ver.minor == 2 && ver.patch >= 5)
        {
            assert_eq!(res.unwrap(), "test_helper.name");
        } else {
            assert!(res
                .err()
                .unwrap()
                .to_string()
                .contains("RedisModule_GetCurrentCommandName is not available"));
        }
    }

    Ok(())
}

#[test]
fn test_helper_info() -> Result<()> {
    const MODULES: [(&str, bool); 4] = [
        ("test_helper", false),
        ("info_handler_macro", false),
        ("info_handler_builder", true),
        ("info_handler_struct", true),
    ];

    MODULES
        .into_iter()
        .try_for_each(|(module, has_dictionary)| {
            let port: u16 = 6483;
            let _guards = vec![start_valkey_server_with_module(module, port)
                .with_context(|| FAILED_TO_START_SERVER)?];
            let mut con =
                get_valkey_connection(port).with_context(|| FAILED_TO_CONNECT_TO_SERVER)?;

            let res: String = redis::cmd("INFO")
                .arg(module)
                .query(&mut con)
                .with_context(|| format!("failed to run INFO {module}"))?;

            assert!(res.contains(&format!("{module}_field:value")));
            if has_dictionary {
                assert!(res.contains("dictionary:key=value"));
            }

            Ok(())
        })
}

#[test]
fn test_test_helper_err() -> Result<()> {
    let port: u16 = 6484;
    let _guards = vec![start_valkey_server_with_module("test_helper", port)
        .with_context(|| FAILED_TO_START_SERVER)?];
    let mut con = get_valkey_connection(port).with_context(|| FAILED_TO_CONNECT_TO_SERVER)?;

    for message in ["\x00\x00", "no crash\x00"] {
        let error = redis::cmd("test_helper.err")
            .arg(message)
            .query::<()>(&mut con)
            .expect_err("test_helper.err should return an error");

        assert_eq!(error.kind(), redis::ErrorKind::ExtensionError);
    }

    Ok(())
}
