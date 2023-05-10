# Validator setup
## overview
Timechain blockchain is a permissionless network, anyone can join and help to verify the blocks. Timechain is based on substrate framework, current consensus is NPoS. So validator can get the reward according to the staking and block production. 
Timechain extends the client functionality like data fetch from external blockchain, send transaction to external blockchain and using TSS to verify any event or state, TSS signatures storage submission and storage in Timechain. All these extended and off-chain functionalities is implemented by chronicle worker, which share the same binary with Timechain node. It is mandatory for Timechain validator to run chronicle worker to involve in off-chain function execution. Chronicle worker also is rewarded according to its contribution.


## hardware
For hardware requirement, reference to the [Polkadot validator hardware](https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot#initial-set-up)

As mentioned overview, Timechain validator also needs to run chronicle node. The requirement for hardware is higher than running a polkadot validator. Other factor is the block slot is 3.6 seconds for Timechain, higher throughput is expected.

### CPU

Recommended:  

- Storage-optimized solutions such as ***non-volatile memory express (NVMe)***
 and ***solid-state disks (SSDs)***
- x86_64 (Intel, AMD) processor with at least 32 physical cores

### RAM

- Recommended: 32 GB

### Disk

- Recommended: 2 TB SSD

## OS
The latest Ubuntu is recommended to run validator.

## Software

### deploy
You can build binary from scratch or download the image with right version from analog docker hub.

#### build binary
- install according to substrate env setup [build env](https://docs.substrate.io/install/) or follow the doc [rust-setup](./rust-setup.md) in the same folder
- run 
```sh
cargo +nightly build --release
```

## account
### account for validator 
[session key](https://docs.substrate.io/deploy/keys-and-network-operations/)

### account for TSS
chronicle worker need to run the TSS service, to sign event data and verify fetched data. 
TODO how to generate TSS account


download the image from analog docker hub with right version.
`docker image pull analog/timechain-node:latest`

## staking 
To become a validator node, you need stake Analog token. The [official instruction](https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot#bond-dot).


## Rosetta service
To interact with external chain, the chronicle worker needs to connect to Rosetta service. You can run your own Rosetta or connect to the entry point provided by Analog.

## start the validator
the command line is like
```
  ./target/debug/timechain-node --validator --port 30335 --base-path /var/validator \
    --ws-port=9947 --rpc-port=9948 --chain mainnet --charlie \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
    --connector-url http://rosetta.analog.one:8081 --connector-blockchain ethereum --connector-network dev \
    >./ind_validators/validator3/out_validator3 2>&1 &
```
To guarantee the node could be automatically started after unexpected exit, you can use daemon tool to start the node.

## monitor 
You need to install monitor system to know the status of validator node. You can check [polkadot node monitor](https://wiki.polkadot.network/docs/maintain-guides-how-to-monitor-your-node)

Analog will extend the service to monitor the chronicle worker. 