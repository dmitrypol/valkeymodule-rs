# Test Shims System Allocator Design

## Goal

Make the `test-shims` Cargo feature automatically enable `enable-system-alloc`.
This ensures code using the in-process Valkey API shims does not accidentally
use the production Valkey allocator outside a Valkey process.

## Design

Declare `enable-system-alloc` as a feature dependency:

```toml
test-shims = ["enable-system-alloc"]
```

The allocator feature remains independently selectable. Existing consumers
that explicitly enable both features remain compatible.

## Verification

Use Cargo's resolved feature output to confirm that selecting `test-shims`
also selects `enable-system-alloc`, then run the relevant test build.

