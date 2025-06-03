// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Primitives.sol)

pragma solidity >=0.8.0;

uint8 constant GMP_VERSION = 0;

/**
 * @dev Maximum size of the GMP payload
 */
uint256 constant MAX_PAYLOAD_SIZE = 0x6000;

/**
 * @dev Tss public key
 * @param yParity public key y-coord parity, the contract converts it to 27/28
 * @param xCoord affine x-coordinate
 */
struct TssKey {
    uint256 xCoord;
    uint8 yParity;
    uint16 numSessions;
}

/**
 * @dev Schnorr signature.
 * OBS: what is actually signed is: keccak256(abi.encodePacked(R, parity, px, nonce, message))
 * Where `parity` is the public key y coordinate stored in the contract, and `R` is computed from `e` and `s` parameters.
 * @param xCoord public key x coordinates, y-parity is stored in the contract
 * @param e Schnorr signature e component
 * @param s Schnorr signature s component
 */
struct Signature {
    uint256 xCoord;
    uint256 e;
    uint256 s;
}

/**
 * @dev GMP payload, this is what the timechain creates as task payload
 * @param source Pubkey/Address of who send the GMP message
 * @param srcNetwork Source chain identifier (for ethereum networks it is the EIP-155 chain id)
 * @param dest Destination/Recipient contract address
 * @param destNetwork Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
 * @param gasLimit gas limit of the GMP call
 * @param nonce Sequence nonce per sender, allows sending two messages with same content
 * @param data message data with no specified format
 */
struct GmpMessage {
    bytes32 source;
    uint16 srcNetwork;
    address dest;
    uint16 destNetwork;
    uint64 gasLimit;
    uint64 nonce;
    bytes data;
}

/**
 * @dev Messages from Timechain take the form of these commands.
 */
enum Command {
    Invalid,
    GMP,
    RegisterShard,
    UnregisterShard,
    SetRoute
}

/**
 * @dev Inbound message from a Timechain
 * @param command Command identifier.
 * @param params Encoded command.
 */
struct GatewayOp {
    /// @dev The command to execute
    Command command;
    /// @dev The Parameters for the command
    bytes params;
}

/**
 * @dev Inbound message from a Timechain
 * @param version Message version, will change if the message format changes.
 * @param batchId Sequence number representing the batch order.
 * @param ops List of operations to execute.
 */
struct Batch {
    /// @dev The batch format version
    uint8 version;
    /// @dev The batch identifier
    uint64 batchId;
    /// @dev The ops to execute
    GatewayOp[] ops;
}

/**
 * @dev A Route represents a communication channel between two networks.
 * @param networkId The id of the provided network.
 * @param gasLimit The maximum amount of gas we allow on this particular network.
 * @param gateway Destination chain gateway address.
 * @param relativeGasPriceNumerator Gas price numerator in terms of the source chain token.
 * @param relativeGasPriceDenominator Gas price denominator in terms of the source chain token.
 */
struct Route {
    uint16 networkId;
    uint64 gasLimit;
    uint128 baseFee;
    bytes32 gateway;
    uint256 relativeGasPriceNumerator;
    uint256 relativeGasPriceDenominator;
    uint64 gasCoef0;
    uint64 gasCoef1;
}

/**
 * @dev Status of a GMP message
 */
enum GmpStatus {
    NOT_FOUND,
    SUCCESS,
    REVERT,
    INSUFFICIENT_FUNDS,
    PENDING
}

/**
 * @dev GmpMessage with EIP-712 GMP ID and callback function encoded.
 * @param eip712hash EIP-712 hash of the `GmpMessage`, which is it's unique identifier
 * @param source Pubkey/Address of who send the GMP message
 * @param srcNetwork Source chain identifier (for ethereum networks it is the EIP-155 chain id)
 * @param dest Destination/Recipient contract address
 * @param destNetwork Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
 * @param gasLimit gas limit of the GMP call
 * @param nonce Sequence nonce per sender, allows sending two messages with same content
 * @param callback encoded callback of `IGmpRecipient` interface, see `IGateway.sol` for more details.
 */
struct GmpCallback {
    bytes32 opHash;
    bytes32 source;
    uint16 srcNetwork;
    address dest;
    uint16 destNetwork;
    uint64 gasLimit;
    uint64 nonce;
    bytes callback;
}

/**
 * @dev Utility functions for primitives
 */
library PrimitiveUtils {
    /**
     * @dev Solidity's reserved location for the free memory pointer.
     * Reference: https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html
     */
    uint256 internal constant ALLOCATED_MEMORY = 0x40;

    /**
     * @dev Read the current allocated size (a.k.a free memory pointer).
     */
    function readAllocatedMemory() internal pure returns (uint256 pointer) {
        assembly ("memory-safe") {
            pointer := mload(ALLOCATED_MEMORY)
        }
    }

    /**
     * @dev Replace the current allocated size by the `newPointer`, and returns the old value stored.
     * CAUTION: Only use this method if you know what you are doing. Make sure you don't overwrite any
     * memory location that is still in use by the current call context.
     */
    function unsafeReplaceAllocatedMemory(uint256 newPointer) internal pure returns (uint256 oldPointer) {
        assembly ("memory-safe") {
            oldPointer := mload(ALLOCATED_MEMORY)
            mstore(ALLOCATED_MEMORY, newPointer)
        }
    }

    /**
     * @dev Hashes three 256-bit words without memory allocation, uses the memory between 0x00~0x60.
     *
     * The reserverd memory region `0x40~0x60` is restored to its previous state after execution.
     * See https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html for more details.
     */
    function hash(uint256 a, uint256 b, uint256 c) internal pure returns (bytes32 h) {
        assembly ("memory-safe") {
            mstore(0x00, a)
            mstore(0x20, b)

            // Backup the free memory pointer
            let freeMemBackup := mload(ALLOCATED_MEMORY)

            mstore(ALLOCATED_MEMORY, c)
            h := keccak256(0x00, 0x60)

            // Restore the free memory pointer
            mstore(ALLOCATED_MEMORY, freeMemBackup)
        }
    }

    /**
     * @dev Returns the smallest of two numbers.
     */
    function min(uint256 x, uint256 y) internal pure returns (uint256) {
        return ternary(x < y, x, y);
    }

    /**
     * @dev Returns the largest of two numbers.
     */
    function max(uint256 x, uint256 y) internal pure returns (uint256) {
        return ternary(x > y, x, y);
    }

    /**
     * @dev If `condition` is true returns `a`, otherwise returns `b`.
     */
    function ternary(bool condition, uint256 a, uint256 b) internal pure returns (uint256 r) {
        // branchless select, works because:
        // b ^ (a ^ b) == a
        // b ^ 0 == b
        //
        // This is better than doing `condition ? a : b` because:
        // - Consumes less gas
        // - Constant gas cost regardless the inputs
        // - Reduces the final bytecode size
        assembly {
            r := xor(b, mul(xor(a, b), condition))
        }
    }

    /**
     * @dev If `condition` is true returns `a`, otherwise returns `b`.
     * see `BranchlessMath.ternary`
     */
    function ternaryU64(bool condition, uint64 a, uint64 b) internal pure returns (uint64 r) {
        assembly {
            r := xor(b, mul(xor(a, b), condition))
        }
    }

    /**
     * @dev Cast a boolean (false or true) to a uint256 (0 or 1) with no jump.
     */
    function toUint(bool b) internal pure returns (uint256 u) {
        assembly ("memory-safe") {
            u := iszero(iszero(b))
        }
    }

    /**
     * @dev Aligns `x` to 32 bytes.
     */
    function align32(uint256 x) internal pure returns (uint256 r) {
        unchecked {
            r = (x + 31) & 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0;
        }
    }

    /**
     * @dev Convert byte count to 256bit word count, rounded up.
     */
    function toWordCount(uint256 byteCount) internal pure returns (uint256 words) {
        assembly {
            words := add(shr(5, byteCount), gt(and(byteCount, 0x1f), 0))
        }
    }

    function toAddress(bytes32 sender) internal pure returns (address) {
        return address(uint160(uint256(sender)));
    }

    function toSender(address addr) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(addr)));
    }

    function messageId(GmpMessage memory message) internal pure returns (bytes32 id) {
        assembly ("memory-safe") {
            // now compute the GmpMessage Type Hash without memory copying
            let offset1 := sub(message, 32)
            let backup1 := mload(offset1)

            mstore(offset1, GMP_VERSION)
            id := keccak256(offset1, 0xe0)
            mstore(offset1, backup1)
        }
    }

    /**
     * @dev Converts the `GmpMessage` into a `GmpCallback` struct, which contains all fields from
     * `GmpMessage`, plus the EIP-712 hash and `IGmpReceiver.onGmpReceived` callback.
     *
     * This method also prevents copying the `message.data` to memory twice, which is expensive if
     * the data is large.
     * Example: using solidity high-level `abi.encode` method does the following.
     *   1. Copy the `message.data` to memory to compute the `GmpMessage` EIP-712 hash.
     *   2. Copy again to encode the `IGmpReceiver.onGmpReceived` callback.
     *
     * Instead we copy it once and use the same memory location to compute the EIP-712 hash and
     * create he `IGmpReceiver.onGmpReceived` callback, unfortunately this requires inline assembly.
     *
     * @param m GmpMessage from calldata to be encoded
     * @param callback `GmpCallback` struct
     */
    function toCallback(GmpMessage calldata m) internal pure returns (GmpCallback memory callback) {
        // |  MEMORY OFFSET  |     RESERVED FIELD     |
        // | 0x0000..0x0020 <- GmpCallback.opHash
        // | 0x0020..0x0040 <- GmpCallback.source
        // | 0x0040..0x0060 <- GmpCallback.srcNetwork
        // | 0x0060..0x0080 <- GmpCallback.dest
        // | 0x0080..0x00a0 <- GmpCallback.destNetwork
        // | 0x00a0..0x00c0 <- GmpCallback.gasLimit
        // | 0x00c0..0x00e0 <- GmpCallback.nonce
        // | 0x00e0..0x0100 <- GmpCallback.callback.offset
        // | 0x0100..0x0120 <- GmpCallback.callback.length
        // | 0x0120..0x0124 <- onGmpReceived.selector (4 bytes)
        // | 0x0124..0x0144 <- onGmpReceived.id
        // | 0x0144..0x0164 <- onGmpReceived.network
        // | 0x0164..0x0184 <- onGmpReceived.source
        // | 0x0184..0x01a4 <- onGmpReceived.nonce
        // | 0x01a4..0x01c4 <- onGmpReceived.data.offset
        // | 0x01c4..0x01e4 <- onGmpReceived.data.length
        // | 0x01e4........ <- onGmpReceived.data
        callback.source = m.source;
        callback.srcNetwork = m.srcNetwork;
        callback.dest = m.dest;
        callback.destNetwork = m.destNetwork;
        callback.gasLimit = m.gasLimit;
        callback.nonce = m.nonce;
        bytes calldata data = m.data;
        callback.callback = abi.encodeWithSignature(
            "onGmpReceived(bytes32,uint128,bytes32,uint64,bytes)",
            callback.opHash,
            callback.srcNetwork,
            callback.source,
            callback.nonce,
            data
        );
        _computeMessageID(callback);
    }

    /**
     * @dev Computes the message ID from the provided `GmpCallback` struct.
     */
    function _computeMessageID(GmpCallback memory callback) private pure {
        bytes memory onGmpReceived = callback.callback;
        callback.opHash = bytes32(uint256(GMP_VERSION));

        bytes32 msgId;
        assembly ("memory-safe") {
            // Compute `keccak256(abi.encode(GMP_VERSION, message.source, ...))`
            msgId := keccak256(callback, 0x00e0)
            // Replace the `id` in `onGmpReceived(uint256 id,...)` in the callback.
            mstore(add(onGmpReceived, 0x24), msgId)
        }

        bytes memory data;
        assembly ("memory-safe") {
            data := add(onGmpReceived, 0xc4)
        }
        bytes32 dataHash = keccak256(data);

        callback.opHash = msgId;
        bytes32 backup = callback.source;
        callback.source = dataHash;
        assembly ("memory-safe") {
            dataHash := keccak256(callback, 0x40)
        }
        callback.opHash = dataHash;
        callback.source = backup;
    }

    function messageId(GmpCallback memory callback) internal pure returns (bytes32 msgId) {
        bytes memory onGmpReceived = callback.callback;
        assembly ("memory-safe") {
            msgId := mload(add(onGmpReceived, 0x24))
        }
    }
}
