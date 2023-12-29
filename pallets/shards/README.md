# Shards Pallet

## Storage:
### Name: 
    ShardIdCounter
### Purpose:
    Contains counter for total shards in network.
    
### Name: 
    ShardNetwork
### Purpose:
    Stores what network given shard is working on.
    
### Name: 
    ShartState
### Purpose:
    Tells the state of shard e.g. created, committed, online, offline

### Name: 
    ShardThreshold
### Purpose:
    Stores the threshold for shard tss signing.

### Name: 
    ShardCommitment
### Purpose:
    Stores shard commitment.
    
### Name: 
    MemberShard
### Purpose:
    Stores shard id against each member of timechain.
    
### Name: 
    ShardMembers
### Purpose:
    Stores members per shard.
    
### Name: 
    PastSigners
### Purpose:
    Stores signers which already did signing for a specific shard. 
    
## Events:
### Name: 
		ShardCreated(ShardId, Network),
### Purpose:
		New shard was created

### Name: 
		ShardCommitted(ShardId, Commitment),
### Purpose:
		Shard commited

### Name: 
		ShardKeyGenTimedOut(ShardId),
### Purpose:
		Shard DKG timed out

### Name: 
		ShardOnline(ShardId, TssPublicKey),
### Purpose:
		Shard completed dkg and submitted public key to runtime

### Name: 
		ShardOffline(ShardId),
### Purpose:
		Shard went offline

## Extrinsics:
### Name: 
    Commit
### Origin:
    ShardMember
### Purpose:
    Submits Commitment and proof of knowledge. 
### Nature:
    Automatic submission by node.
    
### Name:
    ready
### Origin:
    ShardMember
### Purpose:
    Turns shard status online and ready to receive tasks for execution.
### Nature:
    Automatic submission by node.