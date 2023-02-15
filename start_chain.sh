#!/usr/bin/env bash

install_diesel_cli() {
  cargo install diesel_cli 2>/dev/null || true
}

create_database() {
  export PGPASSWORD=postgres
  psql -h localhost -p 5432 -U postgres -w -c "CREATE DATABASE timechain;" 2>/dev/null || true
}

run_migrations() {
  diesel migration run
}

set -e

start_boot_node() {
  echo "Starting boot node..."
  install -d ./ind_validators/validator1
  ./target/debug/timechain-node --validator --base-path ./ind_validators/validator1 --port 30333 --ws-port=9943 --rpc-port=9944 --chain local --alice --node-key 0000000000000000000000000000000000000000000000000000000000000001 >./ind_validators/validator1/out_validator1 2>&1 &
}

start_validator_1() {
  echo "Starting validator 1..."
  install -d ./ind_validators/validator2
  ./target/debug/timechain-node --validator --port 30334 --base-path ./ind_validators/validator2 \
    -ltime=trace --ws-port=9945 --rpc-port=9946 --chain local --bob \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
    >./ind_validators/validator2/out_validator2 2>&1 &
}

start_validator_2() {
  echo "Starting validator 2..."
  install -d ./ind_validators/validator3
  ./target/debug/timechain-node --validator --port 30335 --base-path ./ind_validators/validator3 -lthea=trace  \
    --ws-port=9947 --rpc-port=9948 --chain local --charlie \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
    >./ind_validators/validator3/out_validator3 2>&1 &
}

start_validator_3() {
  echo "Starting validator 2..."
  install -d ./ind_validators/validator4
  ./target/debug/timechain-node --validator --port 30336 --base-path ./ind_validators/validator4 -lthea=trace  \
    --ws-port=9957 --rpc-port=9958 --chain local --eve \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
    >./ind_validators/validator3/out_validator4 2>&1 &
}

kill_nodes() {
  echo "Killing all nodes."
  killall -2 timechain-node
}

SLEEP=10

start_chain() {
  ./purge-chain.sh

  install_diesel_cli
  create_database
  run_migrations

  start_boot_node
  sleep $SLEEP

  start_validator_1
  sleep $SLEEP
  start_validator_2
  sleep $SLEEP
   start_validator_3
  #sleep $SLEEP
  
  #./set-keys.sh
  
  tail -f ./ind_validators/validator1/out_validator1
}

start_chain