<div align="center">

  <h1><code>Timechain Node</code></h1>

  <strong>This repo is for the timechain node, which is part of the analog network. <a href="https://github.com/analog-labs">Analog</a>.</strong>

  <h3>
    <a href="https://analog.one/">Docs</a>
    <span> | </span>
    <a href="mailto:hello@analog.one">Support</a>
  </h3>

</div>

## Features

This repo includes the minimum required components to start a timechain testnet node, inspired by [substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

* Consensus related pallets: Aura & GRANDPA
* Token vesting pallet: analog-vesting store the vesting schedule and the vesting balance of the account.
* Account delegation pallet: pallet-proxy store the delegation relationship between master account and delegator account. Delegate account can submit the task and pay task fee from master account.
* Task metadata storage pallet: pallet-task-metadata store the static configuration of the task.
* Task schedule pallet: pallet-task-schedule store the task and its status. now it support the one-time task, repetitive task and payable task.
* Tesseract sig storage pallet: pallet-tesseract-sig-storage store the TSS shards and the signature collected from TSS workers.
* Task executor worker: run the tasks on chain.
* TSS worker: receive the data fetched from task executor, verify and sign against the data, and send the extrinsic to store the signature if reaches the threshold.

**Notes:** The code is still under active development and not production ready, use it at your own risk.

## Getting Started

Follow the steps below to get started.

### Rust Setup

First, complete the [Dev Docs Installation](https://docs.substrate.io/v3/getting-started/installation/).

### Build and Run

Use the following command to build the node and run it after build successfully:

```sh
cargo build --release
./target/release/timechain-node 
```

## Run Using Docker

if you're on macos add the following to your `.cargo/config`:
```
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"

[env]
CC_x86_64-unknown-linux-musl = "x86_64-linux-musl-gcc"
```

1. Install docker, docker compose and musl using your system package manager.
2. Install the musl target with rustup `rustup target add x86_64-unknown-linux-musl`.
3. Build the docker image with `./build_docker.sh`.
4. Start the timechain validators, connectors and chain nodes with `docker compose up`.
5. Set the validator keys with `./set_keys.sh`.

## Run public testnet

* Start your bootnodes, node key can be generate with command `./target/release/timechain-node key generate-node-key`.
  ```shell
      ./target/release/timechnode --base-path /tmp/bootnode01 --chain ./timechain-staging.json --port 30333 --ws-port 9945 --rpc-port 9933 --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" --validator --rpc-methods Unsafe --name BootNode01
  ```
  
* [Insert session keys](https://substrate.dev/docs/en/tutorials/start-a-private-network/customchain#add-keys-to-keystore)

* Start your initial validators,
  ```shell
    ./target/release/node-template \
    --base-path /tmp/bob \
    --port 30333 \
    --ws-port 9946 \
    --rpc-port 9934 \
    --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
    --validator \
    --connector-url=http://ethereum-connector:8080 \
    --connector-blockchain=ethereum \
    --connector-network=dev \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWDAzWc9PWDapTfx89NmAhxuySLnVU9N62ojYS25Va7gif
  ```


## Upcoming features
* Update the time-chain with Proof of time consensus protocol.
* Attract enough validators from community.
* Enable governance, and remove sudo.
* Enable transfer and other functions.
* Add XCEDT for cross-chain event data transfer.
* Smart Contact SDK.
* Update the time-chain to give proper support to publisher and subscriber so that a user can easily publish time data on chain and use it by subscribing the chain data.

