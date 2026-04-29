#!/usr/bin/env sh
rm -f dump.rdb

FEATURES="enable-system-alloc mockall min-valkey-compatibility-version-8-0 min-redis-compatibility-version-7-2"
cargo test --all --all-targets --no-default-features --features "$FEATURES"
