# Tasks Pallet

Tasks pallet provides a number of extrinsics for managing tasks e.g. `create_task`, `resume_task`, `stop_task`, which are available to timechain users to use.
It also provides extrinsics like `submit_result`, `submit_error`, `submit_signature` etc for timechain nodes to use when processing the tasks of external chains.
`register_gateway` extrinsic is used to register the gateway contract address on which we can process gmp tasks. Following are some stats of a task.
`Read only` tasks phase
`Read(None)`: this is a view call tasks which read the data from the chain.
`GMP` tasks phase
`Sign`: This phase is of a gmp tasks, where the task parameters are signed by each shard member and it is then converted to payable task.
`Payable` tasks phase
`Write(Some(ID))`: In this phase task is ran for changing external chain state and `Some(Id)` user with the id will be responsible for paying for it.
`Read(Some(hash))`: all shard members verifies the signature of hash and tss validate it then sends the result to pallet for task update.

## Storage:
### UnassignedTasks
`Contains a list of unassigned task`
### ShardTasks
`Contains tasks for a specific shard`
### TaskShard
`Contains shard against a specific task`
### NetworkShards
`Contains Shard of a specific network`
### TaskIdCounter
`Contains total number of tasks created`
### Tasks
`List of all tasks inserted in network`
### TaskState
`Contains task status against a task id e.g. Created, Stopped etc`
### TaskPhaseState
`Contains task phase state against a task id e.g. Write(Some(_)), Read(None) etc`
### TaskSignature
`Contains task signature against a task sign phase`
### WritePhaseStart
`Contains BlockNumber to start task write phase`
### TaskRetryCounter
`Counter for consecutively failed attempts for a task`
### TaskCycleState
`Stores task cycle for a task`
### TaskResults
`Stores task results for a task id`
### ShardRegistered
`Stores shards which are registered for gateway contract`
### Gateway
`gateway contract address for the network`

## Events:
### TaskCreated(TaskId),
`the record id that uniquely identify`
### TaskResult(TaskId, TaskCycle, TaskResult),
`Updated cycle status`
### TaskFailed(TaskId, TaskCycle, TaskError),
`Task failed due to more errors than max retry count`
### TaskStopped(TaskId),
`Task stopped by owner`
### TaskResumed(TaskId),
`Task resumed by owner`
### GatewayRegistered(Network, Vec<u8>),
`Gateway registered on network`

## Extrinsics:
### create_task(TaskDescriptorParams)
### Origin:
`Shard Member`
### Purpose:
`Insert task for execution in timechain`

### stop_task(TaskId)
### Origin:
`Task Owner`
### Purpose:
`Stops tasks execution`

### resume_task(TaskId, u64)
### Origin:
`Task Owner`
### Purpose:
`Resumes task execution if its stopped or failed`

### submit_result(TaskId,TaskCycle,TaskResult)
### Origin:
`Shard Member`
### Purpose:
`Submits result for a specific task_id and its cycle`

### submit_error(TaskId,TaskCycle,TaskError)
### Origin:
`Shard Member`
### Purpose:
`Submits error for a specific task_id and its cycle`

### submit_hash(TaskId,TaskCycle,Vec<u8>)
### Origin:
`Shard Member`
### Purpose:
`Submits hash of a payable task for a specific task_id and its cycle`

### submit_signature(TaskId,TssSignature)
### Origin:
`Shard Member`
### Purpose:
`Submits signature for a specific task_id for TSS sign`

### register_gateway(ShardId,Vec<u8>)
### Origin:
`Root User`
### Purpose:
`Register gateway contract against a shard`
