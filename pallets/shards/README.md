# Shards Pallet

Main purpose of this pallet is to create and manages shards in network. It keeps track of shard status (Offline, Online, Committed). 
Contains logic with members goes offline and if enough members are offline turns shard offline and rescheduled the respective tasks.
Also provides the interface to create shard to elections pallet, where election pallet can use it when we have enough members to join.

## Storage:
### ShardIdCounter
`Contains counter for total shards in network`
    
### ShardNetwork
`Stores what network given shard is working on`
    
### ShardState
`Tells the state of shard e.g. created, committed, online, offline`

### ShardThreshold
`Stores the threshold for shard tss signing`

### ShardCommitment
`Stores shard commitment`
    
### MemberShard
`Stores shard id against each member of timechain`
    
### ShardMembers
`Stores members per shard`
    
### PastSigners
`Stores signers which already did signing for a specific shard`
    
## Events:
### ShardCreated(ShardId, Network),
`New shard was created`

### ShardCommitted(ShardId, Commitment),
`Shard commited`

### ShardKeyGenTimedOut(ShardId),
`Shard DKG timed out`

### ShardOnline(ShardId, TssPublicKey),
`Shard completed dkg and submitted public key to runtime`

### ShardOffline(ShardId),
`Shard went offline`

## Extrinsic:
### Commit(ShardId, Commitment, ProofOfKnowledge)
### Origin:
`ShardMember`
### Purpose:
`Submits Commitment and proof of knowledge.`
### Nature:
`Automatic submission by node.`
    
### ready(ShardId) 
### Origin:
`ShardMember`
### Purpose:
`Turns shard status online and ready to receive tasks for execution.`
### Nature:
`Automatic submission by node.`
