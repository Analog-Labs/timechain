#!/bin/sh
rm -rf /home/ethereum/geth
geth init --datadir /home/ethereum /home/ethereum/genesis.json
echo "" | geth \
    --datadir /home/ethereum \
    --allow-insecure-unlock \
    --unlock 0x935A4185494e7A9e0d5A2d609F66bD613616081C \
    --mine \
    --miner.etherbase 0x935A4185494e7A9e0d5A2d609F66bD613616081C \
    --miner.gaslimit 30000000 \
    --maxpeers 0 \
    --ipcdisable \
    --http \
    --http.addr=0.0.0.0 \
    --http.port=8545 \
    --http.vhosts='*' \
    --http.corsdomain='*' \
    --http.api=eth,debug,admin,txpool,web3,net \
    --ws \
    --ws.addr=0.0.0.0 \
    --ws.port=8545 \
    --ws.origins='*' \
    --ws.api=eth,debug,admin,txpool,web3,net \
    --ws.rpcprefix=/
