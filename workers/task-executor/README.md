__Setting up ethereum node and deploying voting contract__

1. Clone this repo: https://github.com/Analog-Labs/chain-connectors
2. checkout `eth-contract-update` branch
3. Go to rosetta-client/Readme.md and follow the instructions for section `Setting up nodes`
4. Follow first 3 steps of `Running voting_contract example` section in the same readme file.
Now we have local node running with voting contract deployed.

__Running testnet__
1. Run ./start_chain.sh
2. set keys.
3. run `node client/nodejs/src/pallet_task_schedule_add_local.js`
4. run `node client/nodejs/src/pallet_task_add_local.js`
you should be receiving logs from contract.
5. register shards.
