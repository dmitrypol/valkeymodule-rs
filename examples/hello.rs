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
    use std::collections::HashMap;
    use valkey_module::TestContext;

    #[test]
    fn wrong_arity_with_no_args() {
        let ctx = TestContext::new(HashMap::new());
        let result = hello_mul(&ctx, vec![]);
        assert!(matches!(result, Err(ValkeyError::WrongArity)));
    }

    #[test]
    fn wrong_arity_with_only_command_name() {
        let ctx = TestContext::new(HashMap::new());
        let cmd = ValkeyString::test("hello.mul");
        let result = hello_mul(&ctx, vec![cmd.safe_clone(&ctx)]);
        assert!(matches!(result, Err(ValkeyError::WrongArity)));
    }
}
