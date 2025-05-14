# DEMO 
## Set Up

### TC 

Start TC

``` sh
scripts/build_docker.sh
docker compose --profile bridge up -d
```

### GMP

Deploy gateways

``` sh
docker compose run --remove-orphans tc-cli --config local-evm-bridge.yaml deploy
```

Register TC network (route) to the gateway at network `2`:

``` sh
docker compose run --remove-orphans tc-cli --config local-evm-bridge.yaml set-tc-route 2 0x49877F1e26d523e716d941a424af46B86EcaF09E
```

Register shard for the network `2`:
    
``` sh
docker compose run --remove-orphans tc-cli --config local-evm-bridge.yaml register-shards 2
```

### ERC20 

We use `AnlogTokenV2` from **Analog-Labs/erc20-token** [`bridge`](https://github.com/Analog-Labs/erc20-token/tree/bridge) branch.

Run tests 

``` sh
cd ../erc20-token
forge clean && forge test
```

Build contract 

``` sh
forge clean && forge build
```

We'll deploy it to network `2` which should have RPC exposed at `8545` port:

1. Find out chronicle's address on target network 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm-bridge.yaml chronicles
```

And save it to env var:

``` sh
# Set this to chronicle's address 
export MINTER=0xC969dEa46d374E26A9f60A6A71c3Fe186050F35F
```

2. Set env vars 

```sh 
# well-known (thus key compromised) Anvil default account(0)
export DEPLOYER=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
# well-known (thus key compromised) Anvil default account(2)
export UPGRADER=0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC
# well-known (thus key compromised) Anvil default account(3)
export PAUSER=0x90F79bf6EB2c4f870365E785982E1f101E93b906
# well-known (thus key compromised) Anvil default account(4)
export UNPAUSER=0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65
export GATEWAY=0x49877F1e26d523e716d941a424af46B86EcaF09E
```

3. Deploy `V1`
(provide the well-known key of `account(0)` of anvil)

``` sh
forge script script/00_Deploy.V1.s.sol --rpc-url="http://localhost:8545" --broadcast -i 1

##### anvil-hardhat
✅  [Success] Hash: 0xdbb41e7ccba67cd14e54c335cc0f2d709586342f17f4ac4f654a411ad77b7da4
Contract Address: 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
Block: 342
Paid: 0.000000000000329646 ETH (329646 gas * 0.000000001 gwei)


##### anvil-hardhat
✅  [Success] Hash: 0x7ba7d7d95cc90d0886414bce186208ea9fb431ade86ad9d7e7f8885d8b0d65ea
Contract Address: 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
Block: 342
Paid: 0.000000000001931458 ETH (1931458 gas * 0.000000001 gwei)

✅ Sequence #1 on anvil-hardhat | Total Paid: 0.000000000002261104 ETH (2261104 gas * avg 0.000000001 gwei)
```
Here we've got 2 addresses: 1st is for the proxy, 2nd is for implementation.

4. Upgrade to `V2` 

Set addresses of the contracts deployed above: 

``` sh
export PROXY=0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
# Teleport-specific
export TIMECHAIN_ROUTE_ID=1000
export MINIMAL_TELEPORT_VALUE=1000000000000
```

(provide the well-known key of `account(2)` of anvil)
``` sh
forge script script/01_Upgrade.V1.V2.s.sol --rpc-url=localhost:8545 --broadcast -i 1

##### anvil-hardhat
✅  [Success] Hash: 0x2fbbf7a8b936a8ca75da75b01824f4724f8668fdf2731ae111b6764a0d62f283
Contract Address: 0x663F3ad617193148711d28f5334eE4Ed07016602
Block: 61
Paid: 0.007274622 ETH (2424874 gas * 3 gwei)


##### anvil-hardhat
✅  [Success] Hash: 0x3582750cdb5febc51ea95c9f3e92cf40019b26984ddde28e0f8c23df38e6ba94
Block: 61
Paid: 0.000112896 ETH (37632 gas * 3 gwei)

✅ Sequence #1 on anvil-hardhat | Total Paid: 0.007387518 ETH (2462506 gas * avg 3 gwei)

```

### Bridge Pallet 

Register a new (or update an existing) network for teleportation at bridge pallet: 

call `bridge/register_network` extrinsic (or `force_update_network`) from sudo with following parameters:

+ network: `2`
+ baseFee: 0
+ data:
  + nonce: 0                         
  + dest: `0x0000000000000000000000009fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` (address of our ERC20 contract (proxy), zero-prefixed to match 32bytes size)

## Flow 

### TC -> ERC20 

Let's teleport some ANLOG to `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` address.

Check that it has zero ANLOG first: 

``` sh
cast call $PROXY "balanceOf(address)(uint256)" 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
0
```

Send `teleport_keep_alive` extrinsic from any account having ANLOG (e.g. `//Eve`):

+ network_id: `2`
+ beneficiary: `0x000000000000000000000000f39Fd6e51aad88F6F4ce6aB8827279cffFb92266`
+ amount: 15000000000000

You should see `bridge.Teleported` event emitted, as well as `task.TaskCreated`, note task_id from it for tracking. 

You can track task status with 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm-bridge.yaml task 13
```

Also, you can monitor tx_hash for the corresponding transaction on the target chain, by getting msg index from the command above, and then querying `batchTxHash` storage of the pallet_tasks.

Once task is successfully completed, ANLOG tokens should have been teleported to target account. 
Let's check that on network `2`:

``` sh
cast call $PROXY "balanceOf(address)(uint256)" 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
15000000000000
```

We can see that the requested amount is successfully teleported.

### ERC20->TC 

Let's now teleport some ERC20 back to TC, to `//Alice` account. 

All you have to do for that is to call [`teleport(bytes32 to, uint256 value)`](https://github.com/Analog-Labs/erc20-token/blob/a93214a85e46a23c127574652e9ec424f5f33c3f/src/AnlogTokenV1.sol#L188) method of our token contract.

First we note current Alice balance in TC, (e.g. is `free: 1,001,000,033,212,589,939` MANLOG).

Then we need to figure out GMP cost for the teleportation message:
``` sh
cast call $PROXY "estimateTeleportCost()" | cast 2d
181255
```

Then we send the teleport tx: 

``` sh
cast send --from 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --unlocked --value 181255 $PROXY "teleport(bytes32,uint256)" 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d 6000000000000
```

here we provide: 

+ 181255 (*10^-18) ethers as a payment for GMP message passing thru gateway 
+ `teleport(bytes32,uint256)` args:
  + Alice's public key on TC (hex-encoded)
    this can be queried with `subkey Alice //inspect`
  + Amount to be teleported 

You can see the tx result and block number as the command's output. Note that block number, once events from the block are reported to TC by chronicle, 
Alice should get the teleported `6` ANLOG to her TC account. 

### Troubleshooting 

1. If you see task as "completed", but tokens are not delivered to dest network: 
   1. Note batch_id of the task: It's x in `SubmitMessage(x)` which you see w `tc-cli task x`;
   2. Query [GmpStatus](https://github.com/Analog-Labs/analog-gmp-examples/blob/00090ef5b83574c5fdaa2a10d428f87e1702cc79/examples/teleport-tokens/BasicERC20.sol) of the message  via querying `gmpInfo(bytes32)` on the gateway:
      
      !note: even if tc-cli report task status as __completed_, message itself could have been reverted. 
      To check actual message status run: 
      ```sh
      cast call 0x49877F1e26d523e716d941a424af46B86EcaF09E "gmpInfo(bytes32)" <msg_id>
      ``` 
   2. Query GMP message tx_hash by batch_id from tasks pallet storage: `batchTxHash(u64)`;
   3. Re-run that tx with trace: 
      `$> cast run <tx_hash>`;
   This will show you tx execution trace 
