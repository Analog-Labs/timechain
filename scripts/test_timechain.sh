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
  sleep 2
done

echo "All keys inserted, initializing test"
node ./js/src/register_shard_eth.js
echo "inserting task"
tsk_registered=$(node ./js/src/add_task_eth.js | sed 's/[^0-9]*//g')
echo "Task registered with id: "$tsk_registered
node ./js/src/await_task_status.js $tsk_registered
