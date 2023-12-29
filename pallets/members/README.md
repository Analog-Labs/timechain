# Members Pallet
## Storage:
### MemberNetwork
`Get member network`

### MemberPeerId
`Get PeerId for member`

### MemberPublicKey
`Get PublicKey for member`

### Heartbeat
`Indicate if member is online or offline`

### MemberStake
`Get stake for member`

## Events:
### RegisteredMember(AccountId, Network, PeerId),
`Member register in timechain`

### HeartbeatReceived(AccountId),
`Heartbeat received from a member`

### MemberOnline(AccountId),
`Member status is changed to online`

### MemberOffline(AccountId),
`Member went offline`

### UnRegisteredMember(AccountId, Network),
`Member is unregistered`

## Extrinsics:
### register_member(Network,PublicKey,PeerId,BalanceOf<T>)
### Origin:
`Shard Member`
### Purpose:
`Registers member in timechain`

### send_heartbeat() 
### Origin:
`Shard Member`
### Purpose:
`Heartbeat sent by member to keep it online`

### unregister_member()
### Origin:
`Shard Member`
### Purpose:
`Member is unregistered from timechain`