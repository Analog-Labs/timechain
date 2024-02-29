// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.7.0 <0.9.0;

import "frost-evm/sol/Schnorr.sol";

/**
 * @dev Required interface of an GMP compliant contract
 */
interface IGmpReceiver {
    /**
     * @dev Handles the receipt of a single GMP message.
     * The contract must verify the msg.sender, it must be the Gateway Contract address.
     *
     * @param id The EIP-712 hash of the message payload, used as GMP unique identifier
     * @param network The chain_id of the source chain who send the message
     * @param source The pubkey/address which sent the GMP message
     * @param payload The message payload with no specified format
     * @return 32 byte result which will be stored together with GMP message
     */
    function onGmpReceived(bytes32 id, uint128 network, bytes32 source, bytes calldata payload)
        external
        payable
        returns (bytes32);
}

/**
 * @dev Required interface of an Gateway compliant contract
 */
interface IGateway {
    /**
     * @dev Emitted when `GmpMessage` is executed.
     */
    event GmpExecuted( // EIP-712 hash of the `GmpPayload`, which is it's unique identifier
        // sender pubkey/address (the format depends on src chain)
        // recipient address
        // GMP message execution status
        // GMP result
    bytes32 indexed id, bytes32 indexed source, address indexed dest, uint256 status, bytes32 result);

    /**
     * @dev Emitted when `UpdateShardsMessage` is executed.
     */
    event KeySetChanged( // EIP-712 hash of the UpdateShardsMessage, zero for sudo
        // shards with keys revoked
        // new shards registered
    bytes32 indexed id, TssKey[] revoked, TssKey[] registered);

    function deposit(bytes32 source, uint16 network) external payable;

    /**
     * Execute GMP message
     */
    function execute(Signature memory signature, GmpMessage memory message)
        external
        returns (uint8 status, bytes32 result);

    /**
     * Update TSS key set
     */
    function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
}

/**
 * @dev Tss public key
 */
struct TssKey {
    uint8 yParity; // public key y-coord parity, the contract converts it to 27/28
    uint256 xCoord; // affine x-coordinate
}

/**
 * @dev Message payload used to revoke or/and register new shards
 */
struct UpdateKeysMessage {
    TssKey[] revoke; // Keys to revoke
    TssKey[] register; // Keys to add
}

/**
 * @dev GMP payload, this is what the timechain creates as task payload
 */
struct GmpMessage {
    bytes32 source; // Pubkey/Address of who send the GMP message
    uint16 srcNetwork; // Source chain identifier (for ethereum networks it is the EIP-155 chain id)
    address dest; // Destination/Recipient contract address
    uint16 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
    uint256 gasLimit; // gas limit of the GMP call
    uint256 salt; // Message salt, useful for sending two messages with same content
    bytes data; // message data with no specified format
}

/**
 * @dev this is what must be signed using the schnorr signature,
 * OBS: what is actually signed is: keccak256(abi.encodePacked(R, parity, px, nonce, message))
 * Where `parity` is the public key y coordinate stored in the contract, and `R` is computed from `e` and `s` parameters.
 */
struct Signature {
    uint256 xCoord; // public key x coordinates, y-parity is stored in the contract
    uint256 e; // Schnorr signature e parameter
    uint256 s; // Schnorr signature s parameter
}

/**
 * @dev Shard info stored in the Gateway Contract
 * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
 * the shard info below 256 bit, so it can be stored in one single storage slot.
 * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
 *
 */
struct KeyInfo {
    uint216 _gap; // gap, so we can use later for store more information about a shard
    uint8 status; // status, 0 = unregisted, 1 = active, 3 = revoked
    uint32 nonce; // shard nonce
}

/**
 * @dev GMP info stored in the Gateway Contract
 * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
 * the attributes 256 bit aligned, ex: nonce, block and status can be read in one storage access.
 * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
 *
 */
struct GmpInfo {
    uint184 _gap; // gap to keep status and blocknumber 256bit aligned
    uint8 status; // message status: NOT_FOUND | PENDING | SUCCESS | REVERT
    uint64 blockNumber; // block in which the message was processed
    bytes32 result; // the result of the GMP message
}

contract SigUtils {
    // EIP-712: Typed structured data hashing and signing
    // https://eips.ethereum.org/EIPS/eip-712
    uint16 internal immutable INITIAL_CHAIN_ID;
    bytes32 internal immutable INITIAL_DOMAIN_SEPARATOR;

    constructor(uint16 networkId) {
        INITIAL_CHAIN_ID = networkId;
        INITIAL_DOMAIN_SEPARATOR = computeDomainSeparator();
    }

    // Reference: https://github.com/transmissions11/solmate/blob/main/src/tokens/ERC20.sol
    function DOMAIN_SEPARATOR() public view virtual returns (bytes32) {
        return block.chainid == INITIAL_CHAIN_ID ? INITIAL_DOMAIN_SEPARATOR : computeDomainSeparator();
    }

    // Computes the EIP-712 domain separador
    function computeDomainSeparator() internal view virtual returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256("Analog Gateway Contract"),
                keccak256("0.1.0"),
                INITIAL_CHAIN_ID,
                address(this)
            )
        );
    }

    // computes the hash of an array of tss keys
    function _getTssKeyHash(TssKey memory tssKey) private pure returns (bytes32) {
        return keccak256(abi.encode(keccak256("TssKey(uint8 yParity,uint256 xCoord)"), tssKey.yParity, tssKey.xCoord));
    }

    // computes the hash of an array of tss keys
    function _getTssKeyArrayHash(TssKey[] memory tssKeys) private pure returns (bytes32) {
        bytes memory keysHashed = new bytes(tssKeys.length * 32);
        uint256 ptr;
        assembly {
            ptr := keysHashed
        }
        for (uint256 i = 0; i < tssKeys.length; i++) {
            bytes32 hash = _getTssKeyHash(tssKeys[i]);
            assembly {
                ptr := add(ptr, 32)
                mstore(ptr, hash)
            }
        }

        return keccak256(keysHashed);
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function _getUpdateKeysHash(UpdateKeysMessage memory message) private pure returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256("UpdateKeysMessage(TssKey[] revoke,TssKey[] register)TssKey(uint8 yParity,uint256 xCoord)"),
                _getTssKeyArrayHash(message.revoke),
                _getTssKeyArrayHash(message.register)
            )
        );
    }

    function getUpdateKeysTypedHash(UpdateKeysMessage memory message) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), _getUpdateKeysHash(message)));
    }

    // computes the hash of an array of tss keys
    function _getGmpHash(GmpMessage memory gmp) private pure returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256(
                    "GmpMessage(bytes32 source,uint128 srcNetwork,address dest,uint128 destNetwork,uint256 gasLimit,uint256 salt,bytes data)"
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

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function getGmpTypedHash(GmpMessage memory message) public view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), _getGmpHash(message)));
    }
}

contract Gateway is IGateway, SigUtils {
    uint8 internal constant GMP_STATUS_NOT_FOUND = 0; // GMP message not processed
    uint8 internal constant GMP_STATUS_SUCCESS = 1; // GMP message executed successfully
    uint8 internal constant GMP_STATUS_REVERTED = 2; // GMP message executed, but reverted
    uint8 internal constant GMP_STATUS_PENDING = 128; // GMP message is pending (used in case of reetrancy)

    uint8 internal constant SHARD_ACTIVE = (1 << 0); // Shard active bitflag
    uint8 internal constant SHARD_Y_PARITY = (1 << 1); // Pubkey y parity bitflag

    uint256 internal constant EXECUTE_GAS_DIFF = 10569; // Measured gas cost difference for `execute`

    // Shard data, maps the pubkey coordX (which is already collision resistant) to shard info.
    mapping(bytes32 => KeyInfo) _shards;

    // GMP message status
    mapping(bytes32 => GmpInfo) _messages;

    // Source address => Source network => Deposit Amount
    mapping(bytes32 => mapping(uint16 => uint256)) _deposits;

    Schnorr _verifier;

    constructor(uint16 _networkId, TssKey[] memory keys) payable SigUtils(_networkId) {
        _registerKeys(keys);

        // emit event
        TssKey[] memory revoked = new TssKey[](0);
        emit KeySetChanged(bytes32(0), revoked, keys);

        _verifier = new Schnorr();
    }

    function gmpInfo(bytes32 id) external view returns (GmpInfo memory) {
        return _messages[id];
    }

    function keyInfo(bytes32 id) external view returns (KeyInfo memory) {
        return _shards[id];
    }

    // Check if shard exists, verify TSS signature and increment shard nonce
    function _verifySignature(Signature memory signature, bytes32 message) private view {
        // Load shard from storage
        KeyInfo storage signer = _shards[bytes32(signature.xCoord)];

        // Verify if shard is active
        uint8 status = signer.status;
        require((status & SHARD_ACTIVE) > 0, "shard key revoked or not exists");

        // Load y parity bit, it must be 27 (even), or 28 (odd)
        // ref: https://ethereum.github.io/yellowpaper/paper.pdf
        uint8 yParity;
        if ((status & SHARD_Y_PARITY) > 0) {
            yParity = 28;
        } else {
            yParity = 27;
        }

        // Verify Signature
        require(
            _verifier.verify(yParity, signature.xCoord, uint256(message), signature.e, signature.s),
            "invalid tss signature"
        );
    }

    // Converts a `TssKey` into an `KeyInfo` unique identifier
    function _tssKeyToShardId(TssKey memory tssKey) private pure returns (bytes32) {
        // The tssKey coord x is already collision resistant
        // if we are unsure about it, we can hash the coord and parity bit
        return bytes32(tssKey.xCoord);
    }

    function _registerKeys(TssKey[] memory keys) private {
        // We don't perform any arithmetic operation, except iterate a loop
        unchecked {
            // Register or activate tss key (revoked keys keep the previous nonce)
            for (uint256 i = 0; i < keys.length; i++) {
                // Validate y-parity bit
                TssKey memory newKey = keys[i];

                // Read shard from storage
                bytes32 shardId = _tssKeyToShardId(newKey);
                KeyInfo storage shard = _shards[shardId];
                uint8 status = shard.status;
                uint32 nonce = shard.nonce;

                // Check if the shard is not active
                require((status & SHARD_ACTIVE) == 0, "already active, cannot register again");

                // Check y-parity
                uint8 yParity = newKey.yParity;
                require(yParity == (yParity & 1), "y parity bit must be 0 or 1, cannot register shard");

                // If nonce is zero, it's a new shard, otherwise it is an existing shard which was previously revoked.
                if (nonce == 0) {
                    // if is a new shard shard, set its initial nonce to 1
                    shard.nonce = 1;
                } else {
                    // If the shard exists, the provided y-parity must match the original one
                    uint8 actualYParity = (status & SHARD_Y_PARITY) > 0 ? 1 : 0;
                    require(
                        actualYParity == yParity,
                        "the provided y-parity doesn't match the existing y-parity, cannot register shard"
                    );
                }

                // store the y-parity in the `KeyInfo`
                if (yParity > 0) {
                    // enable SHARD_Y_PARITY bitflag
                    status |= SHARD_Y_PARITY;
                } else {
                    // disable SHARD_Y_PARITY bitflag
                    status &= ~SHARD_Y_PARITY;
                }

                // enable SHARD_ACTIVE bitflag
                status |= SHARD_ACTIVE;

                // Save new status in the storage
                shard.status = status;
            }
        }
    }

    function _revokeKeys(TssKey[] memory keys) private {
        // We don't perform any arithmetic operation, except iterate a loop
        unchecked {
            // Revoke tss keys
            for (uint256 i = 0; i < keys.length; i++) {
                TssKey memory revokedKey = keys[i];

                // Read shard from storage
                bytes32 shardId = _tssKeyToShardId(revokedKey);
                KeyInfo storage shard = _shards[shardId];

                // Check if the shard exists and is active
                require(shard.nonce > 0, "shard doesn't exists, cannot revoke key");
                require((shard.status & SHARD_ACTIVE) > 0, "cannot revoke a shard key already revoked");

                // Check y-parity
                {
                    uint8 yParity = (shard.status & SHARD_Y_PARITY) > 0 ? 1 : 0;
                    require(yParity == revokedKey.yParity, "invalid y parity bit, cannot revoke key");
                }

                // Disable SHARD_ACTIVE bitflag
                shard.status = shard.status & (~SHARD_ACTIVE); // Disable active flag
            }
        }
    }

    // Register/Revoke TSS keys and emits [`KeySetChanged`] event
    function _updateKeys(bytes32 messageHash, TssKey[] memory keysToRevoke, TssKey[] memory newKeys) private {
        // We don't perform any arithmetic operation, except iterate a loop
        unchecked {
            // Revoke tss keys (revoked keys can be registred again keeping the previous nonce)
            _revokeKeys(keysToRevoke);

            // Register or activate revoked keys
            _registerKeys(newKeys);
        }
        emit KeySetChanged(messageHash, keysToRevoke, newKeys);
    }

    // Register/Revoke TSS keys using shard TSS signature
    function updateKeys(Signature memory signature, UpdateKeysMessage memory message) public {
        bytes32 messageHash = getUpdateKeysTypedHash(message);
        _verifySignature(signature, messageHash);

        // Register shards pubkeys
        _updateKeys(messageHash, message.revoke, message.register);
    }

    // Deposit balance to refund callers of execute
    function deposit(bytes32 source, uint16 network) public payable {
        uint256 depositBefore = _deposits[source][network];
        _deposits[source][network] = depositBefore + msg.value;
    }

    // Execute GMP message
    function _execute(bytes32 payloadHash, GmpMessage memory message) private returns (uint8 status, bytes32 result) {
        // Verify if this GMP message was already executed
        GmpInfo storage gmp = _messages[payloadHash];
        require(gmp.status == GMP_STATUS_NOT_FOUND, "message already executed");

        // Update status to `pending` to prevent reentrancy attacks.
        gmp.status = GMP_STATUS_PENDING;
        gmp.blockNumber = uint64(block.number);

        // The encoded onGmpReceived call
        bytes memory data = abi.encodeWithSelector(
            IGmpReceiver.onGmpReceived.selector, payloadHash, message.srcNetwork, message.source, message.data
        );

        // Execute GMP call
        bytes32[1] memory output;
        bool success;
        uint256 gasLimit = message.gasLimit;
        address dest = message.dest;
        assembly {
            // Using low-level assembly because the GMP is considered executed
            // regardless if the call reverts or not.
            let ptr := add(data, 32)
            let size := mload(data)
            // returns 1 if the call succeed, and 0 if it reverted
            success :=
                call(
                    gasLimit, // call gas limit (defined in the GMP message)
                    dest, // dest address
                    0, // value in wei to transfer (always zero for GMP)
                    ptr, // input memory pointer
                    size, // input size
                    output, // output memory pointer
                    32 // output size (fixed 32 bytes)
                )
        }

        // Get Result
        result = output[0];

        // Update GMP status
        if (success) {
            status = GMP_STATUS_SUCCESS;
        } else {
            status = GMP_STATUS_REVERTED;
        }

        // Persist result and status on storage
        gmp.result = result;
        gmp.status = status;

        // Emit event
        emit GmpExecuted(payloadHash, message.source, message.dest, status, result);
    }

    // Send GMP message using sudo account
    function execute(
        Signature memory signature, // coordinate x, nonce, e, s
        GmpMessage memory message
    ) public returns (uint8 status, bytes32 result) {
        uint256 gasBefore = gasleft();
        require(message.gasLimit >= gasBefore, "gas left below message.gasLimit");
        require(
            _deposits[message.source][message.srcNetwork] > message.gasLimit * tx.gasprice, "deposit below max refund"
        );
        bytes32 messageHash = getGmpTypedHash(message);
        _verifySignature(signature, messageHash);
        (status, result) = _execute(messageHash, message);
        uint256 refund = (gasBefore - gasleft() + EXECUTE_GAS_DIFF) * tx.gasprice;
        _deposits[message.source][message.srcNetwork] = _deposits[message.source][message.srcNetwork] - refund;
        payable(tx.origin).transfer(refund);
    }
}
