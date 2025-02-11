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

We'll use [teleport example contract](https://github.com/Analog-Labs/analog-gmp-examples/blob/00090ef5b83574c5fdaa2a10d428f87e1702cc79/examples/teleport-tokens/BasicERC20.sol). 

Build contract 

``` sh
forge build
```

We'll deploy it to network `2` which should have rpc exposed at `8545` port:

``` sh
forge create --unlocked --from 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --constructor-args-path=./constructor.args.txt examples/teleport-tokens/BasicERC20.sol:BasicERC20 --broadcast

Deployed to:  0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
```

<details>
<summary>`constructor.args.txt`</summary>

```
Token ANLOG 0x49877F1e26d523e716d941a424af46B86EcaF09E 0x0000000000000000000000000000000000000000 1000 0x0000000000000000000000000000000000000000 0
```

</details>


### Bridge Pallet 

Register network for teleportation at bridge pallet: 

call `bridge/register_network extrinsic` from sudo with following parameters:

+ network: `2`
+ baseFee: 0
+ data:
  + nonce: 0                         
  + dest: `0x000000000000000000000000e7f1725E7734CE288F8367e1Bb143E90bb3F0512` (address of our ERC20 contract, zero-prefixed to match 32bytes size)

## Flow 

### TC -> ERC20 

Let's teleport some ANLOG to `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` address.

Check that it has zero ANLOG first: 

``` sh
cast call 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "balanceOf(address)(uint256)" 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
0
```

Send teleport_keep_alive extrinsic from any account having ANLOG (e.g. `//Eve`):

+ network_id: `2`
+ beneficiary: `0x000000000000000000000000f39Fd6e51aad88F6F4ce6aB8827279cffFb92266`
+ amount: 3000000000000

You should see `bridge.Teleported` event emitted, as well as `task.TaskCreated`, note task_id from it for tracking. 

You can track task status with 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml task 13
```

!note: even if tc-cli report task status as __completed_, message itself could have been reverted. 
To check actual message status run: 
```sh
cast call 0x49877F1e26d523e716d941a424af46B86EcaF09E "gmpInfo(bytes32)" <msg_id>
 ``` 





Once task successfully completed, ANLOG tokens should have been teleported to target account. 
Let's check that on network `2`:

``` sh
cast call 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "balanceOf(address)(uint256)" 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

```

#### Troubleshooting 

1. If you see task as "completed", but tokens are not delivered to dest network: 
   1. Note batch_id of the task: It's x in `SubmitMessage(x)` which you see w `tc-cli task x`;
   2. Query [GmpStatus](https://github.com/Analog-Labs/analog-gmp-examples/blob/00090ef5b83574c5fdaa2a10d428f87e1702cc79/examples/teleport-tokens/BasicERC20.sol) of the message  via querying `gmpInfo(bytes32)` on the gateway:
      ```
      cast call 0x49877F1e26d523e716d941a424af46B86EcaF09E "gmpInfo(bytes32)" 7006d4a23a194de7611ffbd974e88fbccc65d223f2742480c365e66dea839668
      ``` 
   2. Query GMP message tx_hash by batch_id from tasks pallet storage: `batchTxHash(u64)`;
   3. Re-run that tx with trace: 
      `$> cast run <tx_hash>`;
   This will show you tx execution trace 


### ERC20 -> TC 
