## **Task Executor**:
This is main executor responsible for executing non payable task. So when user inserts a task we fetch inserted task in this services, executes it, update the status of task and send task data for tss process for further verification.


__Setting up ethereum node and deploying voting contract__

1. Clone this repo: https://github.com/Analog-Labs/chain-connectors
2. Go to rosetta-client/Readme.md and follow the instructions for section `Setting up nodes`
3. Follow first 3 steps of `Running voting_contract example` section in the same readme file.
Now we have local node running with voting contract deployed.

__Running testnet__
1. Run ./start_chain.sh
2. set keys.
3. run `node client/nodejs/src/pallet_task_schedule_add_local.js`
4. run `node client/nodejs/src/pallet_task_add_local.js`
you should be receiving logs from contract.
5. register shards.
