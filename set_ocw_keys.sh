#!/bin/bash

insert_key() {
  printf "setting ocwkey $1\n"
  curl "http://localhost:$2" -H "Content-Type:application/json;charset=utf-8" -d "{
    \"jsonrpc\": \"2.0\",
    \"id\": 1,
    \"method\": \"author_insertKey\",
    \"params\": [
      \"psig\",
      \"//Alice\",
      \"$3\"
    ]
  }"
  printf "\n"
  curl "http://localhost:$2" -H "Content-Type:application/json;charset=utf-8" -d "{
    \"jsonrpc\": \"2.0\",
    \"id\": 1,
    \"method\": \"author_insertKey\",
    \"params\": [
      \"pskd\",
      \"//Alice\",
      \"$3\"
    ]
  }"
  printf "\n"
}

# ethereum keys
insert_key 1 9943 "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
insert_key 2 9945 "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
insert_key 3 9947 "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"

#astar keys
insert_key 4 9949 "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
insert_key 5 9951 "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
insert_key 6 9953 "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
