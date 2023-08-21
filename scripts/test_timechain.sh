#!/bin/bash
TOTAL_INSERTS=0

insert_key() {
  if curl -f "http://localhost:$2" -H "Content-Type:application/json;charset=utf-8" -d "{
    \"jsonrpc\": \"2.0\",
    \"id\": 1,
    \"method\": \"author_insertKey\",
    \"params\": [
      \"time\",
      \"owner word vocal dose decline sunset battle example forget excite gentle waste//$1//time\",
      \"$3\"
    ]
  }"; then
    echo "\n"
    TOTAL_INSERTS=$((TOTAL_INSERTS + 1))
  fi
}

while [ $TOTAL_INSERTS -lt 6 ]
do
  TOTAL_INSERTS=0
  # ethereum keys
  insert_key 1 9943 "0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d"
  insert_key 2 9945 "0xcee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b"
  insert_key 3 9947 "0xa01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02"

  #astar keys
  insert_key 4 9949 "0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f"
  insert_key 5 9951 "0x1843caba7078a699217b23bcec8b57db996fc3d1804948e9ee159fc1dc9b8659"
  insert_key 6 9953 "0x72a170526bb41438d918a9827834c38aff8571bfe9203e38b7a6fd93ecf70d69"
  echo '-----------------------------'
  sleep 5
done

echo "All keys inserted, initializing test"


####### Ethereum testing #########
#registering shard for ethereum
eth_shard=$(node ./js/src/register_shard.js 0 0)
echo "Registered eth, shard "$eth_shard
sleep 5

# deploying ethereum smart contract
echo "deploying contract for Eth"
eth_response=$(./scripts/deploy_test_contract -u "http://127.0.0.1:8080" -b "ethereum" -n "dev")
eth_contract=$(echo $eth_response | grep -oEi 'contract_address [0-9a-zA-Z]+' | grep -oEi '0x[0-9a-zA-Z]+')
eth_block=$(echo $eth_response | grep -oEi 'index: [0-9]+' | grep -oEi '[0-9]+')

echo "Ethereum contract registered with address: "$eth_contract" and block "$eth_block 

# inserting tasks for eth
echo "inserting task for Eth"
eth_tsk_registered=$(node ./js/src/add_task.js 0 $eth_contract $eth_block | sed 's/[^0-9]*//g')
echo "Task registered with id: "$eth_tsk_registered
node ./js/src/await_task_status.js $eth_tsk_registered

####### Astar testing #########
#registering shard for astar
astar_shard=$(node ./js/src/register_shard.js 1 1)
echo "Registered astar, shard "$astar_shard
sleep 5

# deplying astar smart contract
echo "deploying contract for Astar"
astar_response=$(./scripts/deploy_test_contract -u "http://127.0.0.1:8081" -b "astar" -n "dev")
astar_contract=$(echo $astar_response | grep -oEi 'contract_address [0-9a-zA-Z]+' | grep -oEi '0x[0-9a-zA-Z]+')
astar_block=$(echo $astar_response | grep -oEi 'index: [0-9]+' | grep -oEi '[0-9]+')

echo "Astar contract registered with address: "$astar_contract" and block: "$astar_block 

echo "Inserting task for Astar"
astr_tsk_registered=$(node ./js/src/add_task.js 1 $astar_contract $astar_block | sed 's/[^0-9]*//g')
echo "Task registered with id: "$astr_tsk_registered
node ./js/src/await_task_status.js $astr_tsk_registered
