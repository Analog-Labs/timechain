# DEMO 
## Set Up

### TC 

Start TC

``` sh
scripts/build_docker.sh
docker compose --profile evm up -d
```

### GMP
    
Deploy gateways

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml deploy
```

Register TC network (route) to the gateway at network `3`:

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml set-tc-route 3 0x49877F1e26d523e716d941a424af46B86EcaF09E
```

Register shard for the network `3`:

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml register-shards 3
```

### ERC20 

We'll use [teleport example contract](https://github.com/Analog-Labs/analog-gmp-examples/blob/00090ef5b83574c5fdaa2a10d428f87e1702cc79/examples/teleport-tokens/BasicERC20.sol). 

Build contract 

``` sh
forge build
```

We'll deploy it to network `3` which should have rpc exposed at `8545` port

(note: any account having ETH on nw 2 would work for that; can use gmp admin, though getting its pk is not a trivial task..)

To deploy it, we use pre-funded dev account of our evm network node, its address can be queried with 

``` sh
curl --header "Content-Type: application/json" \
        --request POST \
        --data '{"jsonrpc": "2.0", "method": "eth_accounts", "params": [], "id": 0}' \
        http://localhost:8546 | jq ".result"
        
[
  "0x5ccb798f63dbafe140bbfb3bc7a6730f228cd9af"
] 
```

Now we deploy example ERC20 contract 

``` sh
forge create -r localhost:8546 --unlocked --from 0x5ccb798f63dbafe140bbfb3bc7a6730f228cd9af --constructor-args-path=./constructor.args.txt examples/teleport-tokens/BasicERC20.sol:BasicERC20 --broadcast

Deployed to:  0xe7622a00576487E30237004a12ed856A4C2B20Fc
```

### Bridge Pallet 

call register_network extrinsic from sudo with following parameters:

+ network: `3`
+ baseFee: 0
+ data:
  + nonce: 0
  + dest: `0x0000000000000000000000000e7622a00576487E30237004a12ed856A4C2B20Fc` (address of our ERC20 contract, zero-prefixed to match 32bytes size)

## Flow 

### TC -> ERC20 

Let's teleport some ANLOG to `0x5ccb798f63dbafe140bbfb3bc7a6730f228cd9af` address.

Check that it has zero ANLOG first: 

``` sh
cast call 0xe7622a00576487E30237004a12ed856A4C2B20Fc "balanceOf(address)(uint256)" 0x5ccb798f63dbafe140bbfb3bc7a6730f228cd9af -r localhost:8546
0
```

Send teleport_keep_alive extrinsic from any account having ANLOG (e.g. `//Eve`):

+ network_id: `3`
+ beneficiary: `0x0000000000000000000000005ccb798f63dbafe140bbfb3bc7a6730f228cd9af`
+ amount: 1000000000000

You should see `bridge.Teleported` event emitted, as well as `task.TaskCreated`, note task_id from it for tracking. 

You can track task status with 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml task 40
```

***Problem**: task fails w 

``` sh
result: Revert("insufficient gas to execute GMP message"), receipt: TransactionReceipt { transaction_hash: 0xf05aa58a31f0206342d09f0dcfee0d27123749323aa747443da183229b234cae, transaction_index: 0, block_hash: 0x85a6f15ff3a28dc0834ecd1043265f0dcb970c72a645722e060b6632d2b9d72e, block_number: Some(377), from: Some(0x1c14e67873358e6ad9a282714a00445c7787a04c), to: Some(0x49877f1e26d523e716d941a424af46b86ecaf09
```



### ERC20 -> TC 
