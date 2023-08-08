#!/bin/sh

# Build release runtime benchmarks
cargo build --release --features=runtime-benchmarks

# Generate weights
./target/release/timechain-node benchmark pallet \
    --pallet pallet_shards \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./runtime/src/weights/shards.rs

./target/release/timechain-node benchmark pallet \
    --pallet pallet_ocw \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./runtime/src/weights/ocw.rs

./target/release/timechain-node benchmark pallet \
    --pallet pallet_tasks \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./runtime/src/weights/tasks.rs