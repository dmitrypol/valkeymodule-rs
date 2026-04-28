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
    use super::*;
    use valkey_module::ValkeyValue;

    #[test]
    fn test_hello_mul() {
        let ctx = Context::test();
        let args = vec![
            ctx.create_string("hello.mul"),
            ctx.create_string("2"),
            ctx.create_string("3"),
            ctx.create_string("4"),
        ];

        let response = hello_mul(&ctx, args).expect("hello.mul should succeed");

        assert_eq!(
            response,
            ValkeyValue::Array(vec![
                ValkeyValue::Integer(2),
                ValkeyValue::Integer(3),
                ValkeyValue::Integer(4),
                ValkeyValue::Integer(24),
            ])
        );
    }

    #[test]
    fn test_hello_mul_wrong_arity() {
        let ctx = Context::test();
        let args = vec![ctx.create_string("hello.mul")];

        let err = hello_mul(&ctx, args).expect_err("hello.mul should reject missing operands");

        assert!(matches!(err, ValkeyError::WrongArity));
    }

    #[test]
    fn test_hello_mul_rejects_non_integer_input() {
        let ctx = Context::test();
        let args = vec![
            ctx.create_string("hello.mul"),
            ctx.create_string("not-a-number"),
        ];

        let err = hello_mul(&ctx, args).expect_err("hello.mul should reject non-integers");

        assert!(matches!(err, ValkeyError::Str("Couldn't parse as integer")));
    }
}
