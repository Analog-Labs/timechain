## Benchmark
./target/release/timechain-node benchmark pallet  \
--chain dev \
--execution=wasm \
--wasm-execution=compiled \
--pallet pallet_proxy \
--extrinsic set_proxy_account \
--steps 50 \
--repeat 20 \
--output pallets/pallet-proxy/src/weights.rs ;