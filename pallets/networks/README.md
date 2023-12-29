# Networks Pallet
## Storage:
### NetworkIdCounter
`Stores a counter for each network type supported`

### ChainNetworks
`Stores blockchain against its supported types Vec<Networks>`

### NetworkIdToChain
`Stores network_id against (blockchain, network)`

### NetworkIdToChainId
`Stores chain id for specific networkid`

## Events:
### NetworkAdded(NetworkId),
`Added supported network in timechain for task execution for this network`

## Extrinsics:
### add_network(ChainName,ChainNetwork,ChainId)
### Origin:
`Root user`
### Purpose:
`Adds network in timechain so chronicle can serve tasks for this network`