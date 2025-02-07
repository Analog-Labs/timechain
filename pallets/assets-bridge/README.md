# DEMO 
## Set Up

### TC 

Start TC

``` sh
scripts/build_docker.sh bridge
docker compose up --profile evm
```

### GMP

Deploy gateways

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml deploy
```

Register TC network to the gateway at network `3`:

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml tc-cli set-tc-route 3 0x49877F1e26d523e716d941a424af46B86EcaF09E
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
        http://localhost:8545 | jq ".result"
        
[
  "0x86eec87a7f13d3c69944a4eb91a532e41bd0d6b9"
] 
```

Now we deploy example ERC20 contract 

``` sh
forge create -r localhost:8545 --unlocked --from 0x86eec87a7f13d3c69944a4eb91a532e41bd0d6b9 --constructor-args-path=./constructor.args.txt examples/teleport-tokens/BasicERC20.sol:BasicERC20 --broadcast
```

### Bridge Pallet 

call register_network extrinsic from sudo with following parameters:

+ network: 3
+ baseFee: 0
+ data:
  + nonce: 0
  + dest: `0x0000000000000000000000007a33b79C4F61258d84961929152e56978A76523E` (address of our ERC20 contract, zero-prefixed to match 32bytes size)

## Flow 

### TC -> ERC20 

Let's teleport some ANLOG to `0x86eec87a7f13d3c69944a4eb91a532e41bd0d6b9` address.

Check that it has zero ANLOG first: 

``` sh
cast call 0x7a33b79C4F61258d84961929152e56978A76523E "balanceOf(address)(uint256)" 0x86eec87a7f13d3c69944a4eb91a532e41bd0d6b9
0
```

Send teleport_keep_alive extrinsic from any account having ANLOG (e.g. `//Eve`):

+ network_id: 3
+ beneficiary: `0x00000000000000000000000086eec87a7f13d3c69944a4eb91a532e41bd0d6b9`
+ amount: 1000000000000

You should see `bridge.Teleported` event emitted, as well as `task.TaskCreated`, note task_id from it for tracking. 

You can track task status with 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml task 62
```

***Problem**: for some reason that task remains unassigned: 

``` sh
docker compose run --remove-orphans tc-cli --config local-evm.yaml task 62
[+] Creating 1/0
 ✔ Container timechain-tc-cli-run-26d7f993dd06  Removed                                                                                                                                                       0.0s 
2025-02-07T23:14:24.419835Z  INFO tc_cli::slack: not logging to slack: environment variable not found    
2025-02-07T23:14:24.420247Z  INFO tc_cli: main
2025-02-07T23:14:24.458605Z  INFO tc_cli: tc ready in 0s
+------+---------+-------------------------+-------------+------------+-----------+
| task | network | descriptor              | output      | shard      | submitter |
+------+---------+-------------------------+-------------+------------+-----------+
| 62   | 3       | SubmitGatewayMessage(2) | in progress | unassigned |           |
+------+---------+-------------------------+-------------+------------+-----------+
```



### ERC20 -> TC 
