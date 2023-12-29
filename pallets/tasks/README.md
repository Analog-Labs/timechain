# Tasks Pallet

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
