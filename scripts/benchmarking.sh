#!/bin/sh

# Build release runtime benchmarks
cargo build --release --features=runtime-benchmarks

# Collect all pallets needed for benchmarking
# Makes the assumption all pallets are present at: /pallets/$PALLET_NAME
pallets=$(ls pallets/)

# Generate weights
for pallet_name in $pallets; do
    ./target/release/timechain-node benchmark pallet \
        --pallet pallet_$pallet_name \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output ./runtime/src/weights/$pallet_name.rs
done
