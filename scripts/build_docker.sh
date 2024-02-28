#!/bin/sh

set -e

# Check for 'uname' and abort if it is not available.
uname -v > /dev/null 2>&1 || { echo >&2 "ERROR - requires 'uname' to identify the platform."; exit 1; }

# Check for 'docker' and abort if it is not running.
docker info > /dev/null 2>&1 || { echo >&2 "ERROR - requires 'docker', please start docker and try again."; exit 1; }

# Check for 'rustup' and abort if it is not available.
rustup -V > /dev/null 2>&1 || { echo >&2 "ERROR - requires 'rustup' for compile the binaries"; exit 1; }

# Detect host architecture
case "$(uname -m)" in
    x86_64)
        rustTarget='x86_64-unknown-linux-musl'
        muslLinker='x86_64-linux-musl-gcc'
        ;;
    arm64|aarch64)
        rustTarget='aarch64-unknown-linux-musl'
        muslLinker='aarch64-linux-musl-gcc'
        ;;
    *)
        echo >&2 "ERROR - unsupported architecture: $(uname -m)"
        exit 1
        ;;
esac

# Check if the musl linker is installed
# "$muslLinker" --version > /dev/null 2>&1 || { echo >&2 "ERROR - requires '$muslLinker' linker for compile"; exit 1; }

# Check if the rust target is installed
if ! rustup target list | grep -q "$rustTarget"; then
  echo "Installing the musl target with rustup '$rustTarget'"
  rustup target add "$rustTarget"
fi

# Build docker image
cargo build -p timechain-node -p chronicle -p tester --target "$rustTarget" --release 
forge build 
rm -rf target/docker
mkdir -p target/docker

mv "target/$rustTarget/release/timechain-node" target/docker
docker build target/docker -f config/docker/Dockerfile -t analoglabs/timechain-node

mv "target/$rustTarget/release/chronicle" target/docker
docker build target/docker -f config/docker/Dockerfile.chronicle -t analoglabs/chronicle

mv "target/$rustTarget/release/tester" target/docker
docker build target/docker -f config/docker/Dockerfile.tester -t analoglabs/timechain-tester
