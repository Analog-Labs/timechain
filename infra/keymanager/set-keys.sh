#!/bin/bash

insert_key() {
  printf "setting timekey $1\n"
  curl "http://$2:9944" -H "Content-Type:application/json;charset=utf-8" -d "{
    \"jsonrpc\": \"2.0\",
    \"id\": 1,
    \"method\": \"author_insertKey\",
    \"params\": [
      \"time\",
      \"owner word vocal dose decline sunset battle example forget excite gentle waste//$1//time\",
      \"$3\"
    ]
  }"
  printf "\n"
}

/wait

# ethereum keys
insert_key 1 bootnode "0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d"
insert_key 2 validator1 "0xcee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b"
insert_key 3 validator2 "0xa01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02"
# astar keys
insert_key 4 validator3 "0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f"
insert_key 5 validator4 "0x1843caba7078a699217b23bcec8b57db996fc3d1804948e9ee159fc1dc9b8659"
insert_key 6 validator5 "0x72a170526bb41438d918a9827834c38aff8571bfe9203e38b7a6fd93ecf70d69"

sleep 2

cd /nodejs && node src/register_shard_eth.js
cd /nodejs && node src/register_shard_astr.js

