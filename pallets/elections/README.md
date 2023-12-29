# Elections Pallet

Manage unassigned members and shard config

## Storage:
### ShardSize
`Size of each new shard`

### ShardThreshold
`Threshold of each new shard`

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