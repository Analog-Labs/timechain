// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

import {Hashing} from "./utils/Hashing.sol";
import {Schnorr} from "./utils/Schnorr.sol";
import {BranchlessMath} from "./utils/BranchlessMath.sol";
import {GasUtils} from "./utils/GasUtils.sol";
import {ERC1967} from "./utils/ERC1967.sol";
import {UFloat9x56, UFloatMath} from "./utils/Float9x56.sol";
import {RouteStore} from "./storage/Routes.sol";
import {ShardStore} from "./storage/Shards.sol";
import {IGateway} from "./interfaces/IGateway.sol";
import {IUpgradable} from "./interfaces/IUpgradable.sol";
import {IGmpReceiver} from "./interfaces/IGmpReceiver.sol";
import {IExecutor} from "./interfaces/IExecutor.sol";
import {
    Command,
    InboundMessage,
    GatewayOp,
    GmpCallback,
    GmpMessage,
    GmpStatus,
    GmpSender,
    Network,
    Route,
    PrimitiveUtils,
    UpdateKeysMessage,
    Signature,
    TssKey,
    MAX_PAYLOAD_SIZE
} from "./Primitives.sol";
import {NetworkID, NetworkIDHelpers} from "./NetworkID.sol";

abstract contract GatewayEIP712 {
    using NetworkIDHelpers for NetworkID;

    // EIP-712: Typed structured data hashing and signing
    // https://eips.ethereum.org/EIPS/eip-712
    uint16 internal immutable NETWORK_ID;
    address internal immutable PROXY_ADDRESS;

    constructor(uint16 networkId, address gateway) {
        NETWORK_ID = networkId;
        PROXY_ADDRESS = gateway;
    }
}

contract Gateway is IGateway, IExecutor, IUpgradable, GatewayEIP712 {
    using PrimitiveUtils for UpdateKeysMessage;
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for address;
    using BranchlessMath for uint256;
    using UFloatMath for UFloat9x56;
    using ShardStore for ShardStore.MainStorage;
    using RouteStore for RouteStore.MainStorage;
    using RouteStore for RouteStore.NetworkInfo;
    using NetworkIDHelpers for NetworkID;

    /**
     * @dev Selector of `GmpCreated` event.
     * keccak256("GmpCreated(bytes32,bytes32,address,uint16,uint64,uint64,uint64,bytes)");
     */
    bytes32 private constant GMP_CREATED_EVENT_SELECTOR =
        0x081a0b65828c1720ce022ffb992d4a5ec86e2abc4c383acd4029ba8486e41b4f;

    /**
     * @dev The address of the `UniversalFactory` contract, must be the same on all networks.
     */
    address internal constant FACTORY = 0x0000000000001C4Bf962dF86e38F0c10c7972C6E;

    // GMP message status
    mapping(bytes32 => GmpInfo) private _messages;

    // Hash of the previous GMP message submitted.
    mapping(address => uint256) private _nonces;

    // Replay protection mechanism, stores the hash of the executed messages
    // messageHash => shardId
    mapping(bytes32 => bytes32) private _executedMessages;

    /**
     * @dev GMP info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the attributes 256 bit aligned, ex: nonce, block and status can be read in one storage access.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     */
    struct GmpInfo {
        GmpStatus status;
        uint64 blockNumber; // block in which the message was processed
    }

    constructor(uint16 network, address proxy) payable GatewayEIP712(network, proxy) {}

    // EIP-712 typed hash
    function initialize(address proxyAdmin, TssKey[] calldata keys, Network[] calldata networks) external {
        require(PROXY_ADDRESS == address(this) || msg.sender == FACTORY, "only proxy can be initialize");
        ERC1967.setAdmin(proxyAdmin);

        // Register networks
        RouteStore.getMainStorage().initialize(networks, NetworkID.wrap(NETWORK_ID));

        // Register keys
        ShardStore.getMainStorage().registerTssKeys(keys);

        // emit event
        emit ShardsRegistered(keys);
    }

    function nonceOf(address account) external view returns (uint64) {
        return uint64(_nonces[account]);
    }

    function gmpInfo(bytes32 id) external view returns (GmpInfo memory) {
        return _messages[id];
    }

    function keyInfo(bytes32 id) external view returns (ShardStore.ShardInfo memory) {
        ShardStore.MainStorage storage store = ShardStore.getMainStorage();
        return store.get(ShardStore.ShardID.wrap(id));
    }

    function networkId() external view returns (uint16) {
        return NETWORK_ID;
    }

    function networkInfo(uint16 id) external view returns (RouteStore.NetworkInfo memory) {
        return RouteStore.getMainStorage().get(NetworkID.wrap(id));
    }

    /**
     * @dev  Verify if shard exists, if the TSS signature is valid then increment shard's nonce.
     */
    function _verifySignature(Signature calldata signature, bytes32 message) private view {
        // Load shard from storage
        ShardStore.ShardInfo storage signer = ShardStore.getMainStorage().get(signature);

        // Load y parity bit, it must be 27 (even), or 28 (odd)
        // ref: https://ethereum.github.io/yellowpaper/paper.pdf
        uint8 yParity = BranchlessMath.ternaryU8(signer.yParity > 0, 28, 27);

        // Verify Signature
        require(
            Schnorr.verify(yParity, signature.xCoord, uint256(message), signature.e, signature.s),
            "invalid tss signature"
        );
    }

    // Register/Revoke TSS keys using shard TSS signature
    function updateKeys(Signature calldata signature, UpdateKeysMessage calldata message) external {
        // Check if the message was already executed to prevent replay attacks
        bytes32 messageHash = message.eip712hash();
        require(_executedMessages[messageHash] == bytes32(0), "message already executed");

        // Verify the signature and store the message hash
        _verifySignature(signature, messageHash);
        _executedMessages[messageHash] = bytes32(signature.xCoord);

        // Register/Revoke shards pubkeys
        ShardStore.MainStorage storage store = ShardStore.getMainStorage();

        // Revoke tss keys (revoked keys can be registred again keeping the previous nonce)
        store.revokeKeys(message.revoke);

        // Register or activate revoked keys
        store.registerTssKeys(message.register);

        // Emit event
        if (message.revoke.length > 0) {
            emit ShardsUnregistered(message.revoke);
        }

        if (message.register.length > 0) {
            emit ShardsRegistered(message.register);
        }
    }

    /*//////////////////////////////////////////////////////////////
                  GATEWAY OPERATIONS AND COMMANDS
    //////////////////////////////////////////////////////////////*/

    /**
     * @dev Lookup table to find the function pointer for a given command.
     * Different than nested if-else, this has constant gas cost regardless the number of commands.
     *
     * Obs: supports up to 16 commands.
     * See: `_buildCommandsLUT` and `_cmdTableLookup` methods for more details.
     */
    type CommandsLookUpTable is uint256;

    /**
     * @dev Dispatch a single GMP message.
     */
    function _gmpCommand(bytes calldata params) private returns (bytes32 operationHash) {
        require(params.length >= 256, "invalid GmpMessage");
        GmpMessage calldata gmp;
        assembly {
            gmp := add(params.offset, 0x20)
        }
        _checkGmpMessage(gmp);
        // Convert the `GmpMessage` into `GmpCallback`, which is a more efficient representation.
        // see `src/Primitives.sol` for more details.
        GmpCallback memory callback = gmp.intoCallback();
        operationHash = callback.eip712hash;
        _execute(callback);
    }

    /**
     * @dev Register a single shard and returns the GatewayOp hash.
     */
    function _registerShardCommand(bytes calldata params) private returns (bytes32 operationHash) {
        require(params.length == 64, "invalid TssKey");
        TssKey calldata newShard;
        assembly {
            newShard := params.offset
        }
        operationHash = Hashing.hash(newShard.yParity, newShard.xCoord);
        _setShard(newShard);
    }

    /**
     * @dev Removes a single shard from the set.
     */
    function _unregisterShardCommand(bytes calldata params) private returns (bytes32 operationHash) {
        require(params.length == 64, "invalid TssKey");
        TssKey calldata shard;
        assembly {
            shard := params.offset
        }
        operationHash = Hashing.hash(shard.yParity, shard.xCoord);
        _revokeShard(shard);
    }

    /**
     * Cast the command function into a uint256.
     */
    function fnToPtr(function(bytes calldata) internal returns (bytes32) fn) private pure returns (uint256 ptr) {
        assembly {
            ptr := fn
        }
    }

    /**
     * @dev Creates a lookup table to find the function pointer for a given command.
     *
     * Motivation: More efficient than nested if-else, and also guarantees a constant gas overhead for any command, which
     * makes easier to estimate the gas cost necessary to execute the whole batch.
     */
    function _buildCommandsLUT() private pure returns (CommandsLookUpTable) {
        uint256 lookupTable;
        // GMP
        lookupTable = fnToPtr(_gmpCommand) << (uint256(Command.GMP) << 4);
        // RegisterShard
        lookupTable |= fnToPtr(_registerShardCommand) << (uint256(Command.RegisterShard) << 4);
        // UnregisterShard
        lookupTable |= fnToPtr(_unregisterShardCommand) << (uint256(Command.UnregisterShard) << 4);
        return CommandsLookUpTable.wrap(lookupTable);
    }

    /**
     * @dev Get in constant gas the function pointer for the provided command.
     * See `_buildCommandsLUT` for more details.
     */
    function _cmdTableLookup(CommandsLookUpTable lut, Command command)
        private
        pure
        returns (function(bytes calldata) internal returns (bytes32) fn)
    {
        unchecked {
            // Extract the function pointer from the table using the `Command` as index.
            uint256 ptr = CommandsLookUpTable.unwrap(lut) >> (uint256(command) << 4);
            ptr &= 0xffff;

            // Make sure the function pointer is within the code bounds.
            uint256 codeSize;
            assembly {
                codeSize := codesize()
            }
            require(ptr > 0 && ptr < codeSize, "invalid command");

            // Converts the `uint256` back to `function(bytes calldata) internal returns (bytes32) fn`
            assembly {
                fn := ptr
            }
        }
    }

    /**
     * @dev Execute a batch of `GatewayOp` and returns the maximum amount of memory used in bytes and the operations root hash.
     *
     * This method also reuses the same memory space for each command, to prevent the memory to grow and
     * increase the cost exponentially.
     * @return (uint256, bytes32) Returns a tuple containing the maximum amount of memory used in bytes and the operations root hash.
     */
    function _executeCommands(GatewayOp[] calldata operations) private returns (uint256, bytes32) {
        // Track the free memory pointer, to reset the memory after each command executed.
        uint256 freeMemPointer = GasUtils.readAllocatedMemory();
        uint256 maxAllocatedMemory = freeMemPointer;

        // Create the Command LookUp Table
        CommandsLookUpTable lut = _buildCommandsLUT();

        bytes32 operationsRootHash = bytes32(0);
        for (uint256 i = 0; i < operations.length; i++) {
            GatewayOp calldata operation = operations[i];

            // Lookup the command function pointer
            function(bytes calldata) internal returns (bytes32) commandFN = _cmdTableLookup(lut, operation.command);

            // Execute the command
            bytes32 operationHash = commandFN(operation.params);

            // Update the operations root hash
            operationsRootHash =
                Hashing.hash(uint256(operationsRootHash), uint256(operation.command), uint256(operationHash));

            // Restore the memory, to prevent the memory expansion costs to increase exponentially.
            uint256 newFreeMemPointer = GasUtils.unsafeReplaceAllocatedMemory(freeMemPointer);

            // Update the Max Allocated Memory
            maxAllocatedMemory = maxAllocatedMemory.max(newFreeMemPointer);
        }

        // Compute what was the maximum amount of memory used in bytes
        maxAllocatedMemory = maxAllocatedMemory - freeMemPointer;

        return (maxAllocatedMemory, operationsRootHash);
    }

    /**
     * @dev Verify and dispatch messages from the Timechain.
     */
    function batchExecute(Signature calldata signature, InboundMessage calldata message) external {
        uint256 initialGas = gasleft();
        // Add the solidity selector overhead to the initial gas, this way we guarantee that
        // the `initialGas` represents the actual gas that was available to this contract.
        initialGas = initialGas.saturatingAdd(GasUtils.BATCH_SELECTOR_OVERHEAD);

        // Execute the commands and compute the operations root hash
        (, bytes32 rootHash) = _executeCommands(message.ops);
        emit BatchExecuted(message.batchID);

        // Compute the Batch signing hash
        rootHash = Hashing.hash(message.version, message.batchID, uint256(rootHash));
        bytes32 signingHash =
            keccak256(abi.encodePacked("Analog GMP v2", NETWORK_ID, bytes32(uint256(uint160(address(this)))), rootHash));

        // Verify the signature
        _verifySignature(signature, signingHash);

        // Refund the chronicle gas
        unchecked {
            // Extra gas overhead used to execute the refund logic.
            uint256 gasUsed = 7188;

            // Compute the gas used + base cost + proxy overhead
            gasUsed = gasUsed.saturatingAdd(GasUtils.txBaseCost());
            gasUsed = gasUsed.saturatingAdd(GasUtils.proxyOverheadGasCost(uint16(msg.data.length), 0));
            gasUsed = gasUsed.saturatingAdd(initialGas - gasleft());

            // Compute refund amount
            uint256 refund = BranchlessMath.min(gasUsed.saturatingMul(tx.gasprice), address(this).balance);

            // Refund the gas used
            assembly ("memory-safe") {
                pop(call(gas(), caller(), refund, 0, 0, 0, 0))
            }
        }
    }

    function _execute(GmpCallback memory callback) private returns (GmpStatus, bytes32) {
        // Verify if this GMP message was already executed
        GmpInfo storage gmp = _messages[callback.eip712hash];
        require(gmp.status == GmpStatus.NOT_FOUND, "message already executed");

        // Update status to `pending` to prevent reentrancy attacks.
        gmp.status = GmpStatus.PENDING;
        gmp.blockNumber = uint64(block.number);

        // Cap the GMP gas limit to 50% of the block gas limit
        // OBS: we assume the remaining 50% is enough for the Gateway execution, which is a safe assumption
        // once most EVM blockchains have gas limits above 10M and don't need more than 60k gas for the Gateway execution.
        uint256 gasLimit = BranchlessMath.min(callback.gasLimit, block.gaslimit >> 1);
        unchecked {
            // Add `all but one 64th` to the gas needed, as the defined by EIP-150
            // https://eips.ethereum.org/EIPS/eip-150
            uint256 gasNeeded = gasLimit.saturatingMul(64).saturatingDiv(63);
            // to guarantee it was provided enough gas to execute the GMP message
            gasNeeded = gasNeeded.saturatingAdd(10000);
            require(gasleft() >= gasNeeded, "insufficient gas to execute GMP message");
        }

        // Execute GMP call
        bool success;
        bytes32 result;
        {
            address dest = callback.dest;
            bytes memory onGmpReceivedCallback = callback.callback;
            assembly ("memory-safe") {
                // Using low-level assembly because the GMP is considered executed
                // regardless if the call reverts or not.
                mstore(0, 0)
                success :=
                    call(
                        gasLimit, // call gas limit defined in the GMP message or 50% of the block gas limit
                        dest, // dest address
                        0, // value in wei to transfer (always zero for GMP)
                        add(onGmpReceivedCallback, 32), // input memory pointer
                        mload(onGmpReceivedCallback), // input size
                        0, // output memory pointer
                        32 // output size (fixed 32 bytes)
                    )

                // Get Result, reuse data to keep a predictable memory expansion
                result := mload(0)
            }
        }

        // Update GMP status
        GmpStatus status =
            GmpStatus(BranchlessMath.ternary(success, uint256(GmpStatus.SUCCESS), uint256(GmpStatus.REVERT)));

        // Persist gmp execution status on storage
        gmp.status = status;

        // Emit event
        emit GmpExecuted(callback.eip712hash, callback.source, callback.dest, status, result);

        return (status, result);
    }

    /**
     * @dev Check if the GmpMessage network is correct and if the data is within the maximum size.
     */
    function _checkGmpMessage(GmpMessage calldata message) private view {
        // Theoretically we could remove the destination network field
        // and fill it up with the network id of the contract, then the signature will fail.
        require(message.destNetwork == NETWORK_ID, "invalid gmp network");

        // Check if the message data is too large
        require(message.data.length <= MAX_PAYLOAD_SIZE, "msg data too large");
    }

    /**
     * Execute GMP message
     * @param signature Schnorr signature
     * @param message GMP message
     */
    function execute(Signature calldata signature, GmpMessage calldata message)
        external
        returns (GmpStatus status, bytes32 result)
    {
        uint256 initialGas = gasleft();
        // Add the solidity selector overhead to the initial gas, this way we guarantee that
        // the `initialGas` represents the actual gas that was available to this contract.
        initialGas = initialGas.saturatingAdd(GasUtils.EXECUTION_SELECTOR_OVERHEAD);

        // Check GMP Message
        _checkGmpMessage(message);

        // Convert the `GmpMessage` into `GmpCallback`, which is a more efficient representation.
        // see `src/Primitives.sol` for more details.
        GmpCallback memory callback = message.intoCallback();

        // Verify the TSS Schnorr Signature
        _verifySignature(signature, callback.eip712hash);

        // Execute GMP message
        (status, result) = _execute(callback);

        // Refund the chronicle gas
        unchecked {
            // Compute GMP gas used
            uint256 gasUsed = 7188 - 11;
            gasUsed = gasUsed.saturatingAdd(GasUtils.txBaseCost());
            gasUsed = gasUsed.saturatingAdd(GasUtils.proxyOverheadGasCost(uint16(msg.data.length), 64));
            gasUsed = gasUsed.saturatingAdd(initialGas - gasleft());

            // Compute refund amount
            uint256 refund = BranchlessMath.min(gasUsed.saturatingMul(tx.gasprice), address(this).balance);

            assembly ("memory-safe") {
                // Refund the gas used
                pop(call(gas(), caller(), refund, 0, 0, 0, 0))
            }
        }
    }

    /**
     * @dev Send message from this chain to another chain.
     * @param destinationAddress the target address on the destination chain
     * @param routeId the target chain where the contract call will be made
     * @param executionGasLimit the gas limit available for the contract call
     * @param data message data with no specified format
     */
    function submitMessage(address destinationAddress, uint16 routeId, uint256 executionGasLimit, bytes calldata data)
        external
        payable
        returns (bytes32)
    {
        // Check if the message data is too large
        require(data.length <= MAX_PAYLOAD_SIZE, "msg data is too big");

        // Check if the provided parameters are valid
        // See `RouteStorage.estimateWeiCost` at `storage/Routes.sol` for more details.
        RouteStore.NetworkInfo memory route = RouteStore.getMainStorage().get(NetworkID.wrap(routeId));
        (uint256 gasCost, uint256 fee) = route.estimateCost(data, executionGasLimit);
        require(msg.value >= fee, "insufficient tx value");

        // We use 20 bytes for represent the address and 1 bit for the contract flag
        GmpSender source = msg.sender.toSender(false);

        unchecked {
            // Nonce is per sender, it's incremented for every message sent.
            uint64 nextNonce = uint64(_nonces[msg.sender]++);

            // Create GMP message and update nonce
            GmpMessage memory message =
                GmpMessage(source, NETWORK_ID, destinationAddress, routeId, uint64(executionGasLimit), nextNonce, data);

            // Emit `GmpCreated` event without copy the data, to simplify the gas estimation.
            _emitGmpCreated(
                message.eip712hash(),
                source,
                destinationAddress,
                routeId,
                executionGasLimit,
                gasCost,
                nextNonce,
                message.data
            );
        }
    }

    /**
     * @dev Emit `GmpCreated` event without copy the data, to simplify the gas estimation.
     */
    function _emitGmpCreated(
        bytes32 messageID,
        GmpSender source,
        address destinationAddress,
        uint16 destinationNetwork,
        uint256 executionGasLimit,
        uint256 gasCost,
        uint256 nonce,
        bytes memory payload
    ) private {
        // Emit `GmpCreated` event without copy the data, to simplify the gas estimation.
        // the assembly code below is equivalent to:
        // ```solidity
        // emit GmpCreated(prevHash, source, destinationAddress, destinationNetwork, executionGasLimit, gasCost, nonce, data);
        // return prevHash;
        // ```
        assembly {
            let ptr := sub(payload, 0xa0)
            mstore(add(ptr, 0x00), destinationNetwork) // dest network
            mstore(add(ptr, 0x20), executionGasLimit) // gas limit
            mstore(add(ptr, 0x40), gasCost) // gasCost
            mstore(add(ptr, 0x60), nonce) // nonce
            mstore(add(ptr, 0x80), 0xa0) // data offset
            let size := and(add(mload(payload), 31), 0xffffffe0)
            size := add(size, 192)
            log4(ptr, size, GMP_CREATED_EVENT_SELECTOR, messageID, source, destinationAddress)
            mstore(0, messageID)
            return(0, 32)
        }
    }

    /*//////////////////////////////////////////////////////////////
                        FEE AND PAYMENT LOGIC
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Estimate the gas cost of execute a GMP message.
     * @dev This function is called on the destination chain before calling the gateway to execute a source contract.
     * @param networkid The target chain where the contract call will be made
     * @param messageSize Message size
     * @param messageSize Message gas limit
     */
    function estimateMessageCost(uint16 networkid, uint256 messageSize, uint256 gasLimit)
        external
        view
        returns (uint256)
    {
        RouteStore.NetworkInfo memory route = RouteStore.getMainStorage().get(NetworkID.wrap(networkid));

        // Estimate the cost
        return route.estimateWeiCost(uint16(messageSize), gasLimit);
    }

    /**
     * Deposit funds to the gateway contract
     * IMPORTANT: this function must be called only by the administrator!!!!
     */
    function deposit() external payable {}
    receive() external payable {}

    /**
     * Withdraw funds from the gateway contract
     * @param amount The amount to withdraw
     * @param recipient The recipient address
     * @param data The data to send to the recipient (in case it is a contract)
     */
    function withdraw(uint256 amount, address recipient, bytes calldata data) external returns (bytes memory output) {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        // Check if the recipient is a contract
        if (recipient.code.length > 0) {
            bool success;
            (success, output) = recipient.call{value: amount, gas: gasleft()}(data);
            if (!success) {
                assembly ("memory-safe") {
                    revert(add(output, 32), mload(output))
                }
            }
        } else {
            payable(recipient).transfer(amount);
            output = "";
        }
    }

    /*//////////////////////////////////////////////////////////////
                    SHARDS MANAGEMENT METHODS
    //////////////////////////////////////////////////////////////*/

    /**
     * @dev Register a single Shards with provided TSS public key.
     */
    function _setShard(TssKey calldata publicKey) private {
        bool isSuccess = ShardStore.getMainStorage().register(publicKey);
        if (isSuccess) {
            TssKey[] memory keys = new TssKey[](1);
            keys[0] = publicKey;
            emit ShardsRegistered(keys);
        }
    }

    /**
     * @dev Revoke a single shard TSS Key.
     */
    function _revokeShard(TssKey calldata publicKey) private {
        bool isSuccess = ShardStore.getMainStorage().revoke(publicKey);
        if (isSuccess) {
            TssKey[] memory keys = new TssKey[](1);
            keys[0] = publicKey;
            emit ShardsUnregistered(keys);
        }
    }

    /**
     * @dev List all shards.
     */
    function shards() external view returns (TssKey[] memory) {
        return ShardStore.getMainStorage().listShards();
    }

    /**
     * @dev Returns the number of active shards.
     */
    function shardCount() external view returns (uint256) {
        return ShardStore.getMainStorage().length();
    }

    /**
     * @dev Returns a shard by index.
     * - Reverts with `IndexOutOfBounds` if the index is out of bounds.
     */
    function shardAt(uint256 index) external view returns (TssKey memory) {
        (ShardStore.ShardID xCoord, ShardStore.ShardInfo storage shard) = ShardStore.getMainStorage().at(index);
        return TssKey({xCoord: uint256(ShardStore.ShardID.unwrap(xCoord)), yParity: shard.yParity + 2});
    }

    /**
     * @dev Register a single Shards with provided TSS public key.
     */
    function setShard(TssKey calldata publicKey) external {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        _setShard(publicKey);
    }

    /**
     * @dev Register Shards in batch.
     */
    function setShards(TssKey[] calldata publicKeys) external {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        (TssKey[] memory created, TssKey[] memory revoked) = ShardStore.getMainStorage().replaceTssKeys(publicKeys);

        if (created.length > 0) {
            emit ShardsRegistered(created);
        }

        if (revoked.length > 0) {
            emit ShardsUnregistered(revoked);
        }
    }

    /**
     * @dev Revoke a single shard TSS Key.
     */
    function revokeShard(TssKey calldata publicKey) external {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        _revokeShard(publicKey);
    }

    /**
     * @dev Revoke Shards in batch.
     */
    function revokeShards(TssKey[] calldata publicKeys) external {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        TssKey[] memory revokedKeys = ShardStore.getMainStorage().revokeKeys(publicKeys);
        if (revokedKeys.length > 0) {
            emit ShardsUnregistered(revokedKeys);
        }
    }

    /*//////////////////////////////////////////////////////////////
                       LISTING ROUTES AND SHARDS
    //////////////////////////////////////////////////////////////*/

    /**
     * @dev List all routes.
     */
    function routes() external view returns (Route[] memory) {
        return RouteStore.getMainStorage().listRoutes();
    }

    /**
     * @dev Create or update a single route
     */
    function setRoute(Route calldata info) external {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        RouteStore.getMainStorage().createOrUpdateRoute(info);
    }

    /**
     * @dev Create or update an array of routes
     */
    function setRoutes(Route[] calldata values) external {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        require(values.length > 0, "routes cannot be empty");
        RouteStore.MainStorage storage store = RouteStore.getMainStorage();
        for (uint256 i = 0; i < values.length; i++) {
            store.createOrUpdateRoute(values[i]);
        }
    }

    /*//////////////////////////////////////////////////////////////
                               ADMIN LOGIC
    //////////////////////////////////////////////////////////////*/

    function admin() external view returns (address) {
        return ERC1967.getAdmin();
    }

    function setAdmin(address newAdmin) external payable {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        ERC1967.setAdmin(newAdmin);
    }

    // DANGER: This function is for migration purposes only, it allows the admin to set any storage slot.
    function sudoSetStorage(uint256[2][] calldata values) external payable {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");
        require(values.length > 0, "invalid values");

        uint256 prev = 0;
        for (uint256 i = 0; i < values.length; i++) {
            uint256[2] memory entry = values[i];
            // Guarantee that the storage slot is in ascending order
            // and that there are no repeated storage slots
            uint256 key = entry[0];
            require(i == 0 || key > prev, "repeated storage slot");

            // Protect admin and implementation slots
            require(key != uint256(ERC1967.ADMIN_SLOT), "use setAdmin instead");
            require(key != uint256(ERC1967.IMPLEMENTATION_SLOT), "use upgrade instead");

            // Set storage slot
            uint256 value = entry[1];
            assembly {
                sstore(key, value)
            }
            prev = key;
        }
    }

    function upgrade(address newImplementation) external payable {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");

        // Store the address of the implementation contract
        ERC1967.setImplementation(newImplementation);
    }

    function upgradeAndCall(address newImplementation, bytes memory initializer)
        external
        payable
        returns (bytes memory returndata)
    {
        require(msg.sender == ERC1967.getAdmin(), "unauthorized");

        // Store the address of the implementation contract
        ERC1967.setImplementation(newImplementation);

        // Initialize storage by calling the implementation's using `delegatecall`.
        bool success;
        (success, returndata) = newImplementation.delegatecall(initializer);

        // Revert if the initialization failed
        if (!success) {
            assembly ("memory-safe") {
                revert(add(returndata, 32), mload(returndata))
            }
        }
    }
}
