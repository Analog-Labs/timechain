#!/bin/bash

# Navigate to the root directory of the project
cd "$(dirname "$0")/.."

# Load environment variables from .env file
set -a
[ -f .env ] && source .env
set +a

# Function to start a chronicle-eth instance with specific volume
start_chronicle_eth() {
  local volume_suffix=$1
  docker-compose up -d \
    chronicle-eth \
    -v "$(pwd)/config/wallets/timechain_keyfile${volume_suffix}:/etc/timechain_keyfile:ro"
}

# Decide how to start services based on CHAIN_SPEC
case $CHAIN_SPEC in
  "notss")
    # Start only one instance of chronicle-eth
    start_chronicle_eth "1"
    ;;
  "dev")
    # Start three instances of chronicle-eth with different volumes
    start_chronicle_eth "1"
    start_chronicle_eth "2"
    start_chronicle_eth "3"
    ;;
  *)
    echo "Unknown CHAIN_SPEC value: $CHAIN_SPEC"
    exit 1
    ;;
esac