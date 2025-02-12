# DEMO 
## Set Up

### TC 

Start TC

``` sh
scripts/build_docker.sh
docker compose --profile bridge up -d
```

### GMP
> [!NOTE]  
> For EVM node we use _anvil_ with default pre-funded dev accounts. 
> All txs we send from `account(0): 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266`._

Fund GMP admin account (`0x90f3ba0b5861d5b108385c21b544a482ea1bfbf5`): 

``` sh
cast send --from 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --value 100ether 0x90f3ba0b5861d5b108385c21b544a482ea1bfbf5 --unlocked
```

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

We use `AnlogTokenV1` from [feature/support-gmp-bridge](https://github.com/Analog-Labs/erc20-token/blob/95f249bd0a8e05af55c8cca141b75776e17080d0/src/AnlogTokenV1.sol#L1) branch.

Build contract 

``` sh
forge build
```

We'll deploy it to network `2` which should have rpc exposed at `8545` port:
(provide the well-known key of `account(0)` of anvil)

``` sh
forge script script/00_Deploy.s.sol --rpc-url=localhost:8545 --broadcast -i 1

##### anvil-hardhat
✅  [Success] Hash: 0x9218957da686fc63b210ddb3ef148415fe96a089360af670951273d67df9bdd6
Contract Address: 0xA51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0
Block: 80514
Paid: 0.000000000020520928 ETH (2565116 gas * 0.000000008 gwei)


##### anvil-hardhat
✅  [Success] Hash: 0x61d11e51376104da96844149122a2460e926920448df55adc37c46e6d1601c82
Contract Address: 0x0DCd1Bf9A1b36cE34237eEaFef220932846BCD82
Block: 80514
Paid: 0.000000000002637168 ETH (329646 gas * 0.000000008 gwei)
```

Here we've got 2 addresses: 2nd is for the proxy, 1st is for implementation.



### Bridge Pallet 

Register a new (or update an existing) network for teleportation at bridge pallet: 

call `bridge/register_network extrinsic` (or `force_update_network`) from sudo with following parameters:

+ network: `2`
+ baseFee: 0
+ data:
  + nonce: 0                         
  + dest: `0x0000000000000000000000000DCd1Bf9A1b36cE34237eEaFef220932846BCD82` (address of our ERC20 contract, zero-prefixed to match 32bytes size)

## Flow 

### TC -> ERC20 

Let's teleport some ANLOG to `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` address.

Check that it has zero ANLOG first: 

``` sh
cast call 0x0DCd1Bf9A1b36cE34237eEaFef220932846BCD82 "balanceOf(address)(uint256)" 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
0
```

Send teleport_keep_alive extrinsic from any account having ANLOG (e.g. `//Eve`):

+ network_id: `2`
+ beneficiary: `0x000000000000000000000000f39Fd6e51aad88F6F4ce6aB8827279cffFb92266`
+ amount: 15000000000000

You should see `bridge.Teleported` event emitted, as well as `task.TaskCreated`, note task_id from it for tracking. 

You can track task status with 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml task 13
```

Once task is successfully completed, ANLOG tokens should have been teleported to target account. 
Let's check that on network `2`:

``` sh
cast call 0xa513E6E4b8f2a923D98304ec87F64353C4D5C853 "balanceOf(address)(uint256)" 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
15000000000000
```

We can see that the requested amount is successfully teleported.

### ERC20->TC 

Let's now teleport some ERC20 back to TC, to `//Alice` account. 

All you have to do for that is to call [`teleport(bytes32 to, uint256 value)`](https://github.com/Analog-Labs/erc20-token/blob/a93214a85e46a23c127574652e9ec424f5f33c3f/src/AnlogTokenV1.sol#L188) method of our token contract.

First we note current Alice balance in TC, which is `0` ANLOG.

Then we need to figure out GMP cost for the teleportation message:
``` sh
cast call 0x0DCd1Bf9A1b36cE34237eEaFef220932846BCD82 "estimateTeleportCost()" | cast 2d
181255
```

Then we send the teleport tx: 

``` sh
cast send --from 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --unlocked --value 181255 0x0DCd1Bf9A1b36cE34237eEaFef220932846BCD82 "teleport(bytes32,uint256)" 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d 6000000000000
```

note we provide: 

+ 181255 (*10^-18) ethers as a payment for GMP message passing thru gateway 
+ `teleport(bytes32,uint256)` args:
  + Alice's public key on TC (hex-encoded)
    this can be queried with `subkey Alice //inspect`
  + Amount to be teleported 

You can see the tx result and block number as the command's output. Note that block number, once events from the block are reported to TC by chronicle, 
Alice should get her `6` ANLOG to her account. 

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


### ERC20 -> TC 
