use valkey_module::alloc::ValkeyAlloc;
use valkey_module::{valkey_module, Context, ValkeyError, ValkeyResult, ValkeyString};

fn hello_mul(_: &Context, args: Vec<ValkeyString>) -> ValkeyResult {
    if args.len() < 2 {
        return Err(ValkeyError::WrongArity);
    }

    let nums = args
        .into_iter()
        .skip(1)
        .map(|s| s.parse_integer())
        .collect::<Result<Vec<i64>, ValkeyError>>()?;

    let product = nums.iter().product();
    let mut response = nums;
    response.push(product);

    Ok(response.into())
}

//////////////////////////////////////////////////////

valkey_module! {
    name: "hello",
    version: 1,
    allocator: (ValkeyAlloc, ValkeyAlloc),
    data_types: [],
    commands: [
        ["hello.mul", hello_mul, "", 0, 0, 0],
    ],
}

#[cfg(test)]
mod tests {
    use crate::hello_mul;
    use std::collections::HashMap;
    use valkey_module::{Context, ValkeyError, ValkeyString, ValkeyValue};

    #[test]
    fn test_empty_args() {
        // using dummy context as it's not actually used in the command
        let test = hello_mul(&Context::dummy(), vec![]);
        assert!(matches!(test, Err(ValkeyError::WrongArity)))
    }

    #[test]
    fn test_cmd_name_only() {
        let cmd = ValkeyString::test("hello.mul");
        let test = hello_mul(&Context::dummy(), vec![cmd]);
        assert!(matches!(test, Err(ValkeyError::WrongArity)))
    }

    #[test]
    fn test_hello_mul() {
        // using test context even though it's not necessary
        let ctx = Context::test(HashMap::new());
        let cmd = ValkeyString::test("hello.mul");
        let arg1 = ValkeyString::test("3");
        let arg2 = ValkeyString::test("4");
        let test = hello_mul(&ctx, vec![cmd, arg1, arg2]);
        assert!(matches!(
            test,
            Ok(ValkeyValue::Array(values))
                if values == vec![
                    ValkeyValue::Integer(3),
                    ValkeyValue::Integer(4),
                    ValkeyValue::Integer(12),
                ]
        ))
    }
}
