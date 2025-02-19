#!/usr/bin/env bash

set -e
set -x

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
WORKSPACE_ROOT=$SCRIPT_DIR/../
cd $WORKSPACE_ROOT

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

# Evaluate optional environment argument
environment="${1:-develop}"
case "${environment}" in
	mainnet)
		profile=mainnet
		features=default
		;;
	testnet)
		profile=testnet
		features=testnet
		;;
	develop)
		profile=testnet
		features=develop
		;;
	*)
		echo >&2 "ERROR - unsupported environment: ${1}"
		echo >&2 "      - options: mainnet testnet develop"
		echo >&2 "      - default: develop"
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
cargo build -p timechain-node -p chronicle -p tc-cli -p gmp-grpc --target "$rustTarget" --profile "$profile" --features "$features"

forge build --root analog-gmp

mkdir -p $WORKSPACE_ROOT/target/docker/tc-cli
rm -rf $WORKSPACE_ROOT/target/docker/tc-cli/envs
cp -rL $WORKSPACE_ROOT/config/envs target/docker/tc-cli/envs
rm -rf $WORKSPACE_ROOT/target/docker/tc-cli/analog-gmp
cp -r $WORKSPACE_ROOT/analog-gmp $WORKSPACE_ROOT/target/docker/tc-cli/analog-gmp

build_image () {
	local TARGET=$WORKSPACE_ROOT"target/$rustTarget/$profile/$1"
	local CONTEXT=$WORKSPACE_ROOT"target/docker/$1"
	mkdir -p $CONTEXT
	if ! cmp -s $TARGET "$CONTEXT/$1"; then
		cp $TARGET $CONTEXT
		docker build $CONTEXT -f $WORKSPACE_ROOT"config/docker/Dockerfile.$1" -t "analoglabs/$1-$environment"
	fi
}

build_image "timechain-node"
build_image "chronicle"
build_image "tc-cli"
build_image "gmp-grpc"
