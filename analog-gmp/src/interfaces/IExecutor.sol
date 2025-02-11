// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/interfaces/IExecutor.sol)

pragma solidity >=0.8.0;

import {
    InboundMessage,
    Signature,
    GmpMessage,
    TssKey,
    GmpStatus,
    GmpStatus,
    UpdateKeysMessage,
    GmpSender,
    Route
} from "../Primitives.sol";

/**
 * @dev Required interface of an Gateway compliant contract
 */
interface IExecutor {
    /**
     * @dev Emitted when `GmpMessage` is executed.
     * @param id EIP-712 hash of the `GmpPayload`, which is it's unique identifier
     * @param source sender pubkey/address (the format depends on src chain)
     * @param dest recipient address
     * @param status GMP message execution status
     * @param result GMP result
     */
    event GmpExecuted(
        bytes32 indexed id, GmpSender indexed source, address indexed dest, GmpStatus status, bytes32 result
    );

    /**
     * @dev Emitted when a Batch is executed.
     * @param batch batch_id which is executed
     */
    event BatchExecuted(uint64 batch);

    /**
     * @dev Emitted when shards are registered.
     * @param keys registered shard's keys
     */
    event ShardsRegistered(TssKey[] keys);

    /**
     * @dev Emitted when shards are unregistered.
     * @param keys unregistered shard's keys
     */
    event ShardsUnregistered(TssKey[] keys);

    /**
     * @dev List all shards currently registered in the gateway.
     */
    function shards() external returns (TssKey[] memory);

    function setShard(TssKey calldata publicKey) external;

    /**
     * @dev Register Shards in batch.
     */
    function setShards(TssKey[] calldata publicKeys) external;

    /**
     * @dev Revoke a single shard TSS Key.
     */
    function revokeShard(TssKey calldata publicKey) external;

    /**
     * @dev Revoke a single shard TSS Key.
     */
    function revokeShards(TssKey[] calldata publicKey) external;

    /**
     * @dev List all shards currently registered in the gateway.
     */
    function routes() external returns (Route[] memory);

    function setRoute(Route calldata info) external;

    /**
     * @dev Create or update an array of routes
     */
    function setRoutes(Route[] calldata values) external;

    /**
     * Execute operatins in batch
     * @param signature Schnorr signature
     * @param message GMP message
     */
    function batchExecute(Signature calldata signature, InboundMessage calldata message) external;

    /**
     * Execute GMP message
     * @param signature Schnorr signature
     * @param message GMP message
     */
    function execute(Signature calldata signature, GmpMessage calldata message)
        external
        returns (GmpStatus status, bytes32 result);

    /**
     * Deposit funds to the gateway contract
     */
    function deposit() external payable;
}
