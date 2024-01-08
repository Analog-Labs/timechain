# Elections Pallet

Manage unassigned members, creates shard if enough members are available. Contains the Shard Config i.e ShardSize, and ShardThreshold.
Shard config can be set via `ShardConfigSet` extrinsic, default is (3: shard_size, 2: shard_threshold).

## Storage:
### ShardSize
`Number of members each new shard`

### ShardThreshold
`Threshold from shard size whose signature can be validated as valid signature`

### Unassigned
`Unassigned members per network`

## Events:
### ShardConfigSet(u16, u16),
`Set shard config: size, threshold`

## Extrinsics:
### set_shard_config(u16,u16)
### Origin:
`Root User`
### Purpose:
`Set nodes required threshold signatures to make a valid TSS signature for a shard`