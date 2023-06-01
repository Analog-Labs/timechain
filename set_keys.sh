#!/bin/bash

insert_key() {
  printf "setting timekey $1\n"
  curl "http://localhost:$2" -H "Content-Type:application/json;charset=utf-8" -d "{
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

insert_key 1 9943 "0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d"
insert_key 2 9945 "0xcee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b"
insert_key 3 9947 "0xa01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02"
