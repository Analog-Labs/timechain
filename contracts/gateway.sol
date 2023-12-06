// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.7.0 <0.9.0;

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
    function onGmpReceived(bytes32 id, uint128 network, bytes32 source, bytes calldata payload) payable external returns (bytes32);
}

/**
 * @dev Required interface of an Gateway compliant contract
 */
interface IGateway {
    /**
     * @dev Emitted when `GmpMessage` is executed.
     */
    event GmpExecuted(
        bytes32 indexed id,     // EIP-712 hash of the `GmpPayload`, which is it's unique identifier
        bytes32 indexed source, // sender pubkey/address (the format depends on src chain)
        address indexed dest,   // recipient address
        uint256 status,         // GMP message exection status
        bytes32 result          // GMP result
    );

    /**
     * @dev Emitted when `UpdateShardsMessage` is executed.
     */
    event ShardSetChanged(
        bytes32 indexed id,    // EIP-712 hash of the UpdateShardsMessage, zero for sudo
        TssKey[] revoked,      // shards with keys revoked
        TssKey[] registered    // new shards registered
    );

    function updateShards(Signature memory signature, Message memory message) external;

    /**
     * Execute GMP message
     */
    function sendMessage(Signature memory signature, Message memory message) external returns (uint8 status, bytes32 result);
}

/**
 * @dev Components of Schnorr signature, the parity bit is stored in the contract.
 */
struct Signature {
    uint256 e;
    uint256 s;
}

/**
 * @dev Tss public key
 */
struct TssKey {
    uint8 yParity;  // public key y-coord parity, the contract converts it to 27/28
    uint256 xCoord; // affine x-coordinate
}

/**
 * @dev Message used to revoke or/and register new shards
 */
struct UpdateShardsPayload {
    TssKey[] revoke;    // Keys to revoke
    TssKey[] register;  // Keys to add
}

/**
 * @dev GMP payload, this is what the timechain creates as task payload
 */
struct SendMessagePayload {
    bytes32 source;      // Pubkey/Address of who send the GMP message
    uint128 srcNetwork;  // Source chain identifier (for ethereum networks it is the EIP-155 chain id)
    address dest;        // Destination/Recipient contract address
    uint128 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
    uint256 gasLimit;    // gas limit of the GMP call
    uint256 salt;        // Message salt, useful for sending two messages with same content
    bytes data;          // message data with no specified format
}

struct Message {
    uint256 xCoord;
    uint64 nonce;
    bytes payload;
}

/**
 * @dev Shard info stored in the Gateway Contract
 * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
 * the shard info below 256 bit, so it can be stored in one single storage slot.
 * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
 **/
struct ShardInfo {
    uint216 _gap;  // gap, so we can use later for store more information about a shard
    uint8 status;  // status, 0 = unregisted, 1 = active, 3 = revoked
    uint32 nonce;  // shard nonce
}

/**
 * @dev GMP Message info stored in the Gateway Contract
 * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
 * the attributes 256 bit aligned, ex: nonce, block and status can be read in one storage access.
 * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
 **/
struct GmpMessageInfo {
    uint184 _gap;       // gap to keep status and blocknumber 256bit aligned
    uint8 status;       // message status: NOT_FOUND | PENDING | SUCCESS | REVERT
    uint64 blockNumber; // block in which the message was processed
    bytes32 result;     // the result of the GMP message
}

contract SigUtils {
    // EIP-712: Typed structured data hashing and signing
    // https://eips.ethereum.org/EIPS/eip-712
    uint256 internal immutable INITIAL_CHAIN_ID;
    bytes32 internal immutable INITIAL_DOMAIN_SEPARATOR;

    constructor() {
        INITIAL_CHAIN_ID = block.chainid;
        INITIAL_DOMAIN_SEPARATOR = computeDomainSeparator();
    }

    // Reference: https://github.com/transmissions11/solmate/blob/main/src/tokens/ERC20.sol
    function DOMAIN_SEPARATOR() public view virtual returns (bytes32) {
        return block.chainid == INITIAL_CHAIN_ID ? INITIAL_DOMAIN_SEPARATOR : computeDomainSeparator();
    }

    // Computes the EIP-712 domain separador
    function computeDomainSeparator() internal view virtual returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                    keccak256("Analog Gateway Contract"),
                    keccak256("0.1.0"),
                    block.chainid,
                    address(this)
                )
            );
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function getMessageHash(Message memory message)
        public
        view
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                DOMAIN_SEPARATOR(),
                keccak256(
                    abi.encode(
                        keccak256("GmpMessage(uint256 xCoord,uint64 nonce,bytes payload)"),
                        message.xCoord,
                        message.nonce,
                        message.payload,
                    )
                )
            )
        );
    }

    // secp256k1 group order
    uint256 constant public Q = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;

    /**
     * @param message The EIP-712 hash of the message
     * @param parity The public key y-coord parity (27 or 28)
     * @param px Public key x-coord
     * @param e Schnorr signature challenge
     * @param s Schnorr signature
     * @return true if the signature is valid, false if invalid
     */
    function _verifyTssSignature(
        bytes32 message,
        uint8 parity,
        uint256 px,
        uint256 e,
        uint256 s
    ) internal pure returns (bool) {
        // ecrecover = (m, v, r, s);
        uint256 sp = Q - mulmod(s, px, Q);
        uint256 ep = Q - mulmod(e, px, Q);

        if (sp == 0) {
            return false;
        }
        // the ecrecover precompile implementation checks that the `r` and `s`
        // inputs are non-zero (in this case, `px` and `ep`), thus we don't need to
        // check if they're zero.
        address R = ecrecover(bytes32(sp), parity, bytes32(px), bytes32(ep));
        if (R == address(0)) {
            return false;
        }
        return bytes32(e) == keccak256(
            abi.encodePacked(R, parity, px, message)
        );
    }
}

contract Gateway is IGateway, SigUtils {
    uint8 internal constant GMP_STATUS_NOT_FOUND  = 0;   // GMP message not processed
    uint8 internal constant GMP_STATUS_SUCCESS    = 1;   // GMP message executed successfully
    uint8 internal constant GMP_STATUS_REVERTED   = 2;   // GMP message executed, but reverted
    uint8 internal constant GMP_STATUS_PENDING    = 128; // GMP message is pending (used in case of reetrancy)

    uint8 internal constant SHARD_ACTIVE   = (1 << 0);  // Shard active bitflag
    uint8 internal constant SHARD_Y_PARITY = (1 << 1);  // Pubkey y parity bitflag

    // Shard data, maps the pubkey coordX (which is already collision resistant) to shard info.
    mapping (bytes32 => ShardInfo) _shards;

    // GMP message status
    mapping (bytes32 => GmpMessageInfo) _messages;

    constructor() payable {
        _shards[msg.xCoord] = SHARD_ACTIVE | msg.yParity ? SHARD_Y_PARITY : 0;
    }

    function _verifySignature(Signature memory signature, Message memory message) private {
        // Load shard from storage
        ShardInfo storage shard = _shards[message.xCoord];

        // Verify if shard is active
        uint8 status = signer.status;
        require((status & SHARD_ACTIVE) > 0, "shard key revoked or not exists");

        // Verify msg nonce
        uint32 nonce = signer.nonce;
        require(sigNonce == nonce, "shard key revoked or not exists");

        // Load y parity bit, it must be 27 (even), or 28 (odd)
        // ref: https://ethereum.github.io/yellowpaper/paper.pdf
        uint8 yParity;
        if ((status & SHARD_Y_PARITY) > 0) {
            yParity = 28;
        } else {
            yParity = 27;
        }

        bytes32 hash = getMessageHash(message);

        // Verify Signature
        require(
            _verifyTssSignature(
                hash,
                yParity,
                message.xCoord,
                signature.e,
                signature.s
            ),
            "invalid tss signature"
        );

        // Increment shard nonce
        signer.nonce = nonce + 1;
    }

    // Register/Revoke TSS keys
    function updateShards(Signature memory signature, Message memory message) external {
        _verifySignature(signature, message);

        // We don't perform any arithmetic operation, except iterate a loop
        unchecked {
            // Revoke tss keys
            for (uint256 i=0; i < message.revokeKeys.length; i++) {
                TssKey memory revokedKey = message.revokeKeys[i];

                // Read shard from storage
                ShardInfo storage shard = _shards[revokedKey.xCoord];

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

            // Register or activate tss key (revoked keys keep the previous nonce)
            for (uint256 i=0; i < message.registerKeys.length; i++) {
                // Validate y-parity bit
                TssKey memory newKey = message.registerKeys[i];

                // Read shard from storage
                ShardInfo storage shard = _shards[newKey.xCoord];
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
                    require(actualYParity == yParity, "the provided y-parity doesn't match the existing y-parity, cannot register shard");
                }

                // store the y-parity in the `ShardInfo`
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
        emit ShardSetChanged(messageHash, revokeKeys, registerKeys);
    }

    // Execute GMP message
    function sendMessage(Signature memory signature, Message memory message) external returns (uint8 status, bytes32 result) {
        _verifySignature(signature, message);

        bytes32 messageId = keccak256(message.payload);

        // Verify if this GMP message was already executed
        GmpMessageInfo storage gmpInfo = _messages[messageId];
        require(gmpInfo.status == GMP_STATUS_NOT_FOUND, "message already executed");

        // Update status to `pending` to prevent reentrancy attacks.
        gmpInfo.status = GMP_STATUS_PENDING;
        gmpInfo.blockNumber = uint64(block.number);

        // The encoded onGmpReceived call
        uint256 gasLimit = message.gasLimit;
        address dest = message.dest;
        bytes memory data = abi.encodeWithSelector(
            IGmpReceiver.onGmpReceived.selector,
            payloadHash,
            message.srcNetwork,
            message.source,
            message.data
        );

        // Execute GMP call
        bytes32[1] memory output;
        bool success;
        assembly {
            // Using low-level assembly because the GMP is considered executed
            // regardless if the call reverts or not.
            let ptr := add(data, 32)
            let size := mload(data)
            // returns 1 if the call succeed, and 0 if it reverted
            success := call(
                gasLimit, // call gas limit (defined in the GMP message)
                dest,     // dest address
                0,        // value in wei to transfer (always zero for GMP)
                ptr,      // input memory pointer
                size,     // input size
                output,   // output memory pointer
                32        // output size (fixed 32 bytes)
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
        gmpInfo.result = result;
        gmpInfo.status = status;

        // Emit event
        emit GmpExecuted(messageId, message.source, message.dest, status, result);
    }
}
