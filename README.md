<div align="center">

  <h1><code>Timechain Node</code></h1>

  <strong>This is the timechain node, which is part of the analog network. <a href="https://github.com/analog-labs">Analog</a>.</strong>

  <h3>
    <a href="https://analog.one/">Docs</a>
    <span> | </span>
    <a href="https://matrix.to/#/!HzySYSaIhtyWrwiwEV:matrix.org?via=matrix.parity.io&via=matrix.org&via=web3.foundation">Chat</a>
  </h3>

</div>

## Features

This repo includes the minimum required components to start a timechain PoA testnet node, inspired by [substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

* Consensus related pallets: Aura & GRANDPA
* Governance related pallets: membership
* Event data related pallets: transactions

### Membership pallet:
The Membership pallet allows control of membership of a set of AccountIds, useful for managing membership of a collective.

### Transaction Pallet:
The transaction pallet allows user to publish the time data event on the chain.

**Notes:** The code is still under active development and not production ready, use it at your own risk.

## Getting Started

Follow the steps below to get started.

### Rust Setup

First, complete the [Dev Docs Installation](https://docs.substrate.io/v3/getting-started/installation/).

### Build and Run

Use the following command to build the node and run it after build successfully:

```sh
cargo build --release
./target/release/timechain-node --dev
```

## Run public testnet

* Start your bootnodes, node key can be generate with command `./target/release/timechain-node key generate-node-key`.
  ```shell
  ./target/release/timechain-node \
       --node-key <your-node-key> \
       --base-path /tmp/bootnode1 \
       --chain timechain-staging-raw.json \
       --name bootnode1
  ```
* Start your initial validators,
  ```shell
  ./target/release/timechain-node \
      --base-path  /tmp/validator1 \
      --chain   timechain-staging-raw.json \
      --bootnodes  /ip4/<your-bootnode-ip>/tcp/30333/p2p/<your-bootnode-peerid> \
      --port 30336 \
      --ws-port 9947 \
      --rpc-port 9936 \
      --name  validator1 \
      --validator
  ```
* [Insert session keys](https://substrate.dev/docs/en/tutorials/start-a-private-network/customchain#add-keys-to-keystore)

## Upcoming features
* Update the time-chain with Proof of time consensus protocol.
* Attract enough validators from community.
* Enable governance, and remove sudo.
* Enable transfer and other functions.
* Add XCEDT for cross-chain event data transfer.
* Smart Contact SDK.
* Update the time-chain to give proper support to publisher and subscriber so that a user can easily publish time data on chain and use it by subscribing the chain data.

