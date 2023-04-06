## Benchmark
./target/release/timechain-node benchmark pallet  \
--chain dev \
--execution=wasm \
--wasm-execution=compiled \
--pallet task_schedule \
--extrinsic insert_schedule \
--steps 50 \
--repeat 20 \
--output pallets/task-schedule/src/weightsOut.rs ;