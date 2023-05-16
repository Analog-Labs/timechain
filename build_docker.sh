#!/bin/sh
cargo build -p timechain-node --target x86_64-unknown-linux-musl --release
mkdir -p target/release/timechain-node/bin
mv target/x86_64-unknown-linux-musl/release/timechain-node target/release/timechain-node/bin
docker build target/release/timechain-node -f Dockerfile -t analoglabs/timechain-node