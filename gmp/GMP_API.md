# GMP API

## Gateway/Proxy deployment

Currently lohann deploys it himself. The contract should be deployed and
upgraded by devops. Our testing infrastructure needs to be able to deploy
it.

### Details

The gateway proxy needs to know the gateway address of initial deployment
and the gateway needs to know the proxy address. Like in the existing tester
code, the address of the gateway is computed in advance, no changes made.

The upgrade procedure sets a new address, same as existing behaviour.

```
contract GatewayProxy {
	constructor(address implementation) payable;
}

contract Gateway {
		constructor(uint16 networkId, address proxy) payable;
		function upgrade(address newImplementation) external payable;
```

## Managing the gateway

The gateway has an admin that can perform various configuration and operational
tasks. If at any point in the future the admin is no longer required, the admin
address can be set to [0; 20]. This is no different than what is currently
implemented.

```
		function admin() external view returns (address);
		function setAdmin(address admin) external payable;
```

## Shards

For debugging purposes the currently registered shards need to be queried.
Currently

Devops needs to be able to atomically set the registered shards. This is
necessary for robustness. If gateway and timechain are out of sync for whatever
reason, devops needs to be able to take corrective measures.

Deployment uses `setShards`, it no longer relies on initializing a bootstrap
shard in the constructor and then leaving the gateway to its own devices.

```
		function shards() external view returns (TssPublicKey[]);
		function setShards(TssPublicKey[] memory shards) external payable;
```

## Routes

When sending a message the message price needs to be computed. The gateway
already contains this logic in the `estimateMessageCost` method.

```
		function estimateMessageCost(uint16 networkid, uint256 messageSize, uint256 gasLimit) external view returns (uint256);
```

To make this computation a `Route` is required. This is the necessary data like
`relativeGasPrice`, almost identical to what is currently implemented. Some weird
fields like mortality which were completely unused in both timechain and gateway
were removed, the route contains rust types instead of solidity types, the backend
translates it to solidity types.


Routes are now set after deployment. Previously the route was half initialized
during gateway deployment, so you had to precompute all the addresses of all
the network gateways and then submit a transaction again to set the route arguments.
This also didn't allow for registering new routes after the gateway was deployed.

```
		function routes() external view returns (Route[]);
		function setRoute(Route memory route) external;
```

## Executing batches

The chronicles need to execute batches produced by the timechain. A batch contains a
list of commands for the gateway to execute. The batch is signed and the shard key is
provided for signature verification. This is the same as what was implemented, except
that the message format needs to be understood.

```
		function execute(TssSignature memory signature, uint256 xCoord, bytes memory message) external;
```

While executing a batch one or more events are emitted for the timechain to pick up the
changes and synchronize its state.

```
		event ShardRegistered(TssPublicKey key);

		event ShardUnregistered(TssPublicKey key);

		event MessageExecuted(
			bytes32 indexed id,
			bytes32 indexed source,
			address indexed dest,
			uint256 status,
			bytes32 result
		);

		event BatchExecuted(
			uint64 batch,
		);
```

## Receiving messages

When a message is received an event is emitted, like it works today.

```
		event MessageReceived(
			bytes32 indexed id,
			GmpMessage msg
		);
```

## Testing infrastructure

Previously we had a voter contract that was used for testing, leaking the evm in all
our testing processes. Instead the GMP API provides a testing interface that can be
used. The tester gets deployed with the gateway as an argument and supports sending
and receiving messages via the gateway.

```
	contract GmpTester {
		constructor(address gateway);
		function sendMessage(GmpMessage msg) payable;
		event MessageReceived(GmpMessage msg);
	}
```
