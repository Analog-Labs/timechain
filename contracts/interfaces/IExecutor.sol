// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (contracts/interfaces/IGateway.sol)

pragma solidity ^0.8.20;

/**
 * @dev A contract that executes GMP messages and updates TSS keys
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
    event GmpExecuted(bytes32 indexed id, bytes32 indexed source, address indexed dest, uint256 status, bytes32 result);

    /**
     * @dev Emitted when `UpdateShardsMessage` is executed.
     * @param id EIP-712 hash of the `UpdateShardsMessage`
     * @param revoked shard's keys revoked
     * @param registered shard's keys registered
     */
    event KeySetChanged(bytes32 indexed id, TssKey[] revoked, TssKey[] registered);

    /**
     * @dev Tss public key
     * @param yParity public key y-coord parity, the contract converts it to 27/28
     * @param xCoord affine x-coordinate
     */
    struct TssKey {
        uint8 yParity;
        uint256 xCoord;
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
     * @param salt Message salt, useful for sending two messages with same content
     * @param data message data with no specified format
     */
    struct GmpMessage {
        bytes32 source;
        uint16 srcNetwork;
        address dest;
        uint16 destNetwork;
        uint256 gasLimit;
        uint256 salt;
        bytes data;
    }

    /**
     * @dev Message payload used to revoke or/and register new shards
     * @param revoke Shard's keys to revoke
     * @param register Shard's keys to register
     */
    struct UpdateKeysMessage {
        TssKey[] revoke;
        TssKey[] register;
    }

    /**
     * Execute GMP message
     * @param signature Schnorr signature
     * @param message GMP message
     */
    function execute(Signature memory signature, GmpMessage memory message)
        external
        returns (uint8 status, bytes32 result);

    /**
     * Update TSS key set
     * @param signature Schnorr signature
     * @param message Shard's keys to register and revoke
     */
    function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
}

/**
 * @dev Required interface of an Gateway compliant contract
 */
library IGatewayEIP712 {
    // computes the hash of an array of tss keys
    function eip712hash(IExecutor.TssKey memory tssKey) internal pure returns (bytes32) {
        return keccak256(abi.encode(keccak256("TssKey(uint8 yParity,uint256 xCoord)"), tssKey.yParity, tssKey.xCoord));
    }

    // computes the hash of an array of tss keys
    function eip712hash(IExecutor.TssKey[] memory tssKeys) internal pure returns (bytes32) {
        bytes memory keysHashed = new bytes(tssKeys.length * 32);
        uint256 ptr;
        assembly {
            ptr := keysHashed
        }
        for (uint256 i = 0; i < tssKeys.length; i++) {
            bytes32 hash = eip712hash(tssKeys[i]);
            assembly {
                ptr := add(ptr, 32)
                mstore(ptr, hash)
            }
        }

        return keccak256(keysHashed);
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function eip712hash(IExecutor.UpdateKeysMessage memory message) internal pure returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256("UpdateKeysMessage(TssKey[] revoke,TssKey[] register)TssKey(uint8 yParity,uint256 xCoord)"),
                eip712hash(message.revoke),
                eip712hash(message.register)
            )
        );
    }

    function eip712TypedHash(IExecutor.UpdateKeysMessage memory message, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, eip712hash(message)));
    }

    // computes the hash of an array of tss keys
    function eip712hash(IExecutor.GmpMessage memory gmp) internal pure returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256(
                    "GmpMessage(bytes32 source,uint16 srcNetwork,address dest,uint16 destNetwork,uint256 gasLimit,uint256 salt,bytes data)"
                ),
                gmp.source,
                gmp.srcNetwork,
                gmp.dest,
                gmp.destNetwork,
                gmp.gasLimit,
                gmp.salt,
                keccak256(gmp.data)
            )
        );
    }

    function eip712TypedHash(IExecutor.GmpMessage memory message, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, eip712hash(message)));
    }
}
