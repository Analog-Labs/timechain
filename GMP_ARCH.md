# Overview

The GMP system is built from multiple components. The timechain, a substrate-based chain, the
chronicles, the `tc-cli` command line tool to manage the timechain and chronicles and a gateway
deployed on every target blockchain.

The chronicles communicate with external chains via the gmp interface. The gmp interface is implemented
by a backend which uses a connector to deploy, manage and use the gateway, while providing a chain
agnostic abstraction.

A group of chronicles form a shard to create threshold signatures and execute tasks assigned by the
timechain.

# Configuration
Timechain is configured in a `config.yaml` file. Secrets are stored in a `.env` file. These are
stored in the `config/envs/$ENVIRONMENT` directory.

To get dynamic price data `tc-cli` is used, the output is also stored in the environment directory
as a `prices.csv` file.
```
tc-cli --config ./config/envs/development/config.yaml fetch-token-price-data
```

# Chronicle configuration
Chronicle is configured by command line flags. Most important ones are the `--network-id`, `--target-url`,
`--timechain-url`, `--target-keyfile`, `--timechain-keyfile`. If the keyfiles don't exist a new key is
generated. The target url and keyfile are passed to a gmp backend which the chronicle uses to interface
with a target chain.

During startup the chronicle waits for a network to be registered on the timechain with the supplied
network id and waits for the timechain and target accounts to have a minimum balance.

To manage a swarm of chronicles the `tc-cli` can be used. The urls of the chronicles are read from
`config.yaml` and the chronicle managment interface is used to get all the necessary information about
the chronicle like public keys.

```
tc-cli --config ./config/envs/development/config.yaml chronicles
```

# Deployment
Once the timechain nodes, chronicles and rpc nodes are started, the timechain is configured with the
deploy command.

```
tc-cli --config ./config/envs/development/config.yaml deploy
```

Deployment performs the following steps:
1. global timechain config is set
- shard size
- shard threshold
2. networks are deployed
- gateway is deployed
- network is registered in the timechain
- minimum balance is deposited in the gateway
3. gateway routes are registered
4. register chronicles in the timechain
- chronicle minimum timechain balance is transfered
- chronicle minimum target chain balance is transfered
- chronicle is registered on timechain, stake is provided by tc-cli

## Gateway deployment on EVM chains
The deployment of the gateway on an evm chain is performed by the gmp-evm backend. This involves
deploying three smart contracts.

1. the universal factory
this is the contract that will later deploy the proxy, ensuring that the proxy has the same address
on every evm chain.
2. the gateway proxy
this is the contract that manages gateway updates. It forwards all calls to the actuall gateway contract.
3. the gateway
this is the contract implementing the gateway logic


After the networks, routes and chronicles are automatically configured the timechain starts
the shard creation process.

# Shard creation

1. chronicles check every finalized block what shards they belong to
2. once they're part of a new shard, they generate their tss secret share
3. the commitment of their secret share is submitted on timechain
4. after all shards committed, the timechain computes the shard public key
and chronicles compute their tss signing share
5. shards submit that they successfully completed dkg using the ready extrinsic
6. once all shards are ready the shard is online and ready to start processing tasks

# Heartbeat mechanism

shards regularly submit a heartbeat to notify the timechain that they are online. if threshold
members miss a heartbeat, the shard goes offline. the election process starts and a new shard
is created. all existing tasks get reassigned to online shards.

# Shard registration

After the bootstrap shards complete their key generation and are online, the deployment is
completed by registering the shards with the gateway.

```
tc-cli --config ./config/envs/development/config.yaml register-shards $NETWORK
```

# Calculating message cost
The message price is calculated using the route information registered in the gateway. The
gateway implements an `estimateMessageCost` method.

TODO: lohann

# Creating a gmp message
A smart contract on the source chain submits a message to the gateway. The gateway emits a
message received event.

You can test this using the following commands:
```
tc-cli --config ./config/envs/development/config.yaml deploy-tester $NETWORK
tc-cli --config ./config/envs/development/config.yaml send-message $NETWORK $TESTER $DEST $DEST_ADDR
tc-cli --config ./config/envs/development/config.yaml message-trace $NETWORK $MESSAGE_ID
```

# Reading events
Every network has a read events task that reads events from the gateway. The gateway emits the
following events:

- shard registered
- shard unregistered
- message received
- message executed
- batch executed

# Submitting batches
The timechain creates a batch respecting the per network batch gas limit. A submit batch task is
created and assigned to a shard, a member of the shard is assigned as the batch submitter. The
batch is tss signed and the submitter submits the batch with a valid signature. The batch submitter
is expected to eventually succeed and keep resubmitting the transaction until it is finalized. The
gateway is not permitted to throw any errors on batch submission.

# Reimbusting the batch transaction cost
Submitting a batch has a transaction cost. We reimburst it to chronicles to ensure they don't
deplete their target chain token balances.

# Task rewards
Chronicle members get 2 ANLG tokens. The purpose is to ensure that chronicle timechain balance
doesn't deplete and they always have sufficient funds to submit transactions to the timechain.

# Gateway rebalancing
If messages are more expensive to send from chain A to chain B or if chain A sends more messages
to chain B than chain B to chain A, eventually the chain A gateway will have a large balance and
the chain B gateway will have a low balance. When this happens tokens are withdrawn, exchanged and
deposited using the following commands:

```
tc-cli --config ./config/envs/development/config.yaml withdraw-funds $NETWORK $AMOUNT $ADDRESS
```
