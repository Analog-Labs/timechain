#!/bin/sh

# Build release runtime benchmarks
cargo build --release --features=runtime-benchmarks

# Generate weights
./target/release/timechain-node benchmark pallet \
    --pallet pallet_shards \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./pallets/shards/src/weights.rs

./target/release/timechain-node benchmark pallet \
    --pallet pallet_ocw \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./pallets/ocw/src/weights.rs

./target/release/timechain-node benchmark pallet \
    --pallet pallet_tasks \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./pallets/tasks/src/weights.rs