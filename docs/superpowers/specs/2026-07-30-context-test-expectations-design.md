# Context Test Expectations Design

## Goal

Replace the fixture-`HashMap` form of `Context::test` with a strict,
Mockall-style expectation API while preserving existing production command
handler signatures.

The initial version covers only the `Context` operations already supported by
the current test shims.

## Public API

`Context::test()` returns an owning `TestContext`. `TestContext` dereferences to
`Context`, allowing existing handlers to continue accepting `&Context`.

Tests configure behavior through `expect_*` methods:

```rust
let mut ctx = Context::test();

ctx.expect_get_client_id()
    .times(2)
    .returning(|| 42);

ctx.expect_get_client_name_by_id()
    .withf(|id| *id == 0)
    .returning(|_| Err(ValkeyError::Str("missing")));

ctx.expect_get_client_name_by_id()
    .withf(|id| *id == 42)
    .returning(|_| Ok(ValkeyString::test("client")));

let result = get_client_name(&ctx, vec![]);
ctx.checkpoint();
```

The initial expectation surface is:

- `expect_get_client_id`
- `expect_get_client_name_by_id`
- `expect_set_client_name_by_id`
- `expect_get_client_username_by_id`
- `expect_get_client_cert`
- `expect_deauthenticate_and_close_client_by_id`
- `expect_get_current_user`
- `expect_set_module_options`
- `expect_authenticate_client_with_acl_user`

Higher-level convenience methods such as `get_client_name()` remain ordinary
compositions of their lower-level operations. For example, `get_client_name()`
uses the expectations for `get_client_id()` and `get_client_name_by_id()`.

`create_string` remains built-in test infrastructure and does not require an
expectation in the initial version.

## Expectation Semantics

Each expectation:

- Defaults to exactly one required call.
- Supports `.times(n)` to change the exact required call count.
- Matches all arguments when `.withf(...)` is omitted.
- Supports `.withf(...)` for argument matching.
- Requires `.returning(...)` with an `FnMut` response closure.

Each method may have multiple expectations. Calls select the first registered
expectation whose matcher accepts the arguments and whose call allowance is not
exhausted.

An unconfigured method, unmatched arguments, a missing return closure, or an
extra call panics immediately with the method name and formatted arguments.

`checkpoint()` verifies that all required calls occurred and then clears the
satisfied expectations so the context can be reused. Dropping `TestContext`
performs final verification unless the thread is already unwinding, which
avoids aborting from a second panic.

Return types mirror the public `Context` methods where practical, including
`Status`, `ValkeyString`, and `ValkeyResult<ValkeyString>`.

## Ownership and Data Flow

`TestContext` owns the production-shaped context and stable mock state:

```rust
pub struct TestContext {
    context: Context,
    state: Box<TestContextState>,
}
```

The `Context` raw pointer points to the boxed `TestContextState` through the
opaque `RedisModuleCtx` pointer type. Moving `TestContext` is safe because the
box allocation remains stable.

The process-global `RedisModule_*` pointers continue to be installed once with
`std::sync::Once`. Each shim casts its received context pointer back to the
corresponding `TestContextState`, so separate tests have isolated expectations
even when the Rust test runner executes them in parallel.

The initial `TestContext` is single-threaded and is not `Send` or `Sync`.
Supporting calls from background threads is outside this design.

The mock context state is reclaimed when `TestContext` drops. Existing fake
`ValkeyString` allocation and retain/free behavior remains unchanged; correcting
fake string ownership is separate work.

## FFI Panic Safety

Expectation failures and user-provided return closures may panic. A panic must
not unwind through an `extern "C"` shim because that can abort the process.

For each supported operation:

1. The FFI shim catches the panic with `catch_unwind`.
2. The shim stores the panic payload in `TestContextState`.
3. The shim returns a harmless value appropriate for its C signature.
4. The public `Context` wrapper checks for a pending test panic immediately
   after the raw FFI call.
5. The wrapper resumes unwinding from ordinary Rust code.

The strict immediate-panic guarantee applies to calls made through public
`Context` methods. Direct invocation of raw `RedisModule_*` function pointers is
not part of the supported expectation API.

## Implementation Boundaries

The implementation uses a small custom expectation engine rather than exposing
Mockall-generated types. This keeps Mockall out of the public dependency graph,
preserves concrete `Context` handler signatures, and provides direct
`ctx.expect_*()` methods.

The change is limited to:

- The test-context owner and typed expectation engine.
- The existing context test shims.
- Pending-panic checks in the supported `Context` wrapper methods.
- Migration of existing tests from fixture maps to expectations.

It does not add expectations for unsupported `Context` APIs or refactor command
handlers to traits or generics.

## Verification

Tests must cover:

- Configured return values for every supported method.
- Multiple ordered expectations for one method.
- Default and explicit call counts.
- Argument match success and failure.
- Unconfigured method calls.
- Missing return closures.
- Calls exceeding their expected count.
- Unsatisfied expectations at `checkpoint()` and drop.
- Panic propagation without aborting through FFI.
- `checkpoint()` followed by configuring new expectations.
- Isolation between parallel test contexts.
- Existing example command handlers migrated from fixture maps.

The crate must compile and test both with its normal feature set and with
`test-shims` enabled alongside a supported minimum Valkey/Redis compatibility
feature.
