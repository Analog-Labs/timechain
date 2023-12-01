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
     * @param sender The pubkey/address which sent the GMP message
     * @param payload The message payload with no specified format
     * @return 32 byte result which will be stored together with GMP message
     */
    function onGmpReceived(bytes32 id, uint128 network, bytes32 sender, bytes calldata payload) payable external returns (bytes32);
}

/**
 * @dev Required interface of an Gateway compliant contract
 */
interface IGateway {
    /**
     * @dev Emitted when `GmpMessage` is executed.
     */
    event GmpExecuted(
        bytes32 indexed id,      // EIP-712 hash of the `GmpPayload`, which is it's unique identifier 
        bytes32 indexed source,  // sender pubkey/address (the format depends on src chain)
        address indexed dest,    // recipient address
        uint256 status,          // GMP message exection status
        bytes32 result           // GMP result
    );

    /**
     * @dev Emitted when `UpdateShardsMessage` is executed.
     */
    event ShardSetChanged(
        bytes32 indexed id,    // EIP-712 hash of the UpdateShardsMessage, zero for sudo
        TssKey[] revoked,      // shards with keys revoked
        TssKey[] registered    // new shards registered
    );

    /**
     * Execute GMP message
     */
    function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result);

    /**
     * Update TSS key set
     */
    function updateTSSKeys(Signature memory signature, UpdateShardsMessage memory message) external;
}

/**
 * @dev Components of Schnorr signature, the parity bit is stored in the contract.
 */
struct Signature {
    uint256 xCoord; // affine x-coordinate, the parity bit is stored in the contract. 
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
struct UpdateShardsMessage {
    uint32 nonce;       // shard's nonce to prevent replay attacks
    TssKey[] revoke;    // Keys to revoke
    TssKey[] register;  // Keys to add
}

/**
 * @dev GMP payload, this is what the timechain creates as task payload
 */
struct GmpPayload {
    bytes32 source;      // Pubkey/Address of who send the GMP message
    uint128 srcNetwork;  // Source chain identifier (it's the EIP-155 chain_id for ethereum networks)
    address dest;        // Destination/Recipient contract address
    uint128 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
    uint256 gasLimit;    // gas limit of the GMP call
    uint256 salt;        // Message salt, useful for sending two messages with same content
    bytes data;          // message data with no specified format
}

/**
 * @dev GMP message, this is what the shard signs
 */
struct GmpMessage {
    uint32 nonce;
    GmpPayload payload;
}

/**
 * @dev Shard info stored in the Gateway Contract
 * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
 * the shard info below 256 bit, so it can be stored in one single storage slot.
 * reference: 
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

    // computes the hash of an array of tss keys
    function _getTssKeyHash(TssKey memory tssKey)
        internal
        pure
        returns (bytes32)
    {
        return
            keccak256(
                abi.encode(
                    keccak256("TssKey(uint8 yParity,uint256 xCoord)"),
                    tssKey.yParity,
                    tssKey.xCoord
                )
            );
    }

    // computes the hash of an array of tss keys
    function _getTssKeyArrayHash(TssKey[] memory tssKeys)
        private
        pure
        returns (bytes32)
    {
        return
            keccak256(
                abi.encode(
                    keccak256("TssKey(uint8 yParity,uint256 x)[]"),
                    tssKeys
                )
            );
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function getUpdateShardsMessageTypedHash(UpdateShardsMessage memory message)
        public
        view
        returns (bytes32)
    {
        return
            keccak256(
                abi.encodePacked(
                    "\x19\x01",
                    DOMAIN_SEPARATOR(),
                    keccak256(
                        abi.encode(
                            keccak256("UpdateShardsMessage(uint256 nonce,TssKey[] revoke,TssKey[] register)"),
                            message.nonce,
                            _getTssKeyArrayHash(message.revoke),
                            _getTssKeyArrayHash(message.register)
                        )
                    )
                )
            );
    }

    // computes the hash of an array of tss keys
    function _getGmpPayloadHash(GmpPayload memory gmp)
        internal
        pure
        returns (bytes32)
    {
        return
            keccak256(
                abi.encode(
                    keccak256(
                        "GMPPayload(bytes32 source,uint96 srcNetwork,address dest,uint96 destNetwork,bytes32 sender,uint256 gasLimit,uint256 value,uint256 salt,bytes data)"
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
    function getGmpMessageTypedHash(GmpMessage memory message)
        public
        view
        returns (bytes32 messageHash, bytes32 payloadHash)
    {
        payloadHash = _getGmpPayloadHash(message.payload);
        messageHash = keccak256(
            abi.encodePacked(
                "\x19\x01",
                DOMAIN_SEPARATOR(),
                keccak256(
                    abi.encode(
                        keccak256("GmpMessage(uint32 nonce,GmpPayload payload)"),
                        message.nonce,
                        payloadHash
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
     * @return `bytes4(keccak256("onGmpReceive(uint128,bytes32,bytes)"))` if GMP is allowed
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

        require(sp != 0);
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
    uint8 internal constant GMP_STATUS_PENDING    = 255; // GMP message is pending (used in case of reetrancy)

    uint8 internal constant SHARD_ACTIVE   = (1 << 0);  // Shard active bitflag
    uint8 internal constant SHARD_Y_PARITY = (1 << 1);  // Pubkey y parity bitflag

    // Owner of this contract, who can execute sudo operations
    address _owner;

    // Shard data, maps the pubkey coordX (which is already collision resistant) to shard info.
    mapping (bytes32 => ShardInfo) _shards;

    // GMP message status
    mapping (bytes32 => GmpMessageInfo) _messages;

    constructor() payable {
        _owner = msg.sender;
    }

    function getGMPMessage(bytes32 id) public returns (GmpMessageInfo memory) {
        return _messages[id];
    }

    function getShard(bytes32 id) public returns (ShardInfo memory) {
        return _shards[id];
    }

    // Check if shard exists, verify TSS signature and increment shard nonce
    function _processSignature(Signature memory signature, bytes32 message, uint32 sigNonce) private {
        // Load shard from storage
        bytes32 shardId = _signatureToShardId(signature);
        ShardInfo storage signer = _shards[shardId];

        // Verify if shard is active
        uint8 status = signer.status;
        require((status & SHARD_ACTIVE) > 0, "shard key revoked or not exists");

        // Verify msg nonce
        uint32 nonce = signer.nonce;
        require(sigNonce == nonce, "shard key revoked or not exists");

        // Increment shard nonce
        signer.nonce = nonce + 1;

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
            _verifyTssSignature(
                message,
                yParity,
                signature.xCoord,
                signature.e,
                signature.s
            ),
            "invalid tss signature"
        );
    }

    // Transfer the ownership of this contract to another account
    function transferOwnership(address newOwner) external returns (bool) {
        require(msg.sender == _owner, "not autorized");
        _owner = newOwner;
        return true;
    }

    // Converts a `TssKey` into an `ShardInfo` unique identifier
    function _tssKeyToShardId(TssKey memory tssKey) private pure returns (bytes32) {
        // The tssKey coord x is already collision resistant
        // if we are unsure about it, we can hash the coord and parity bit
        return bytes32(tssKey.xCoord);
    }

    // Converts a `Signature` into an `ShardInfo` unique identifier
    function _signatureToShardId(Signature memory signature) private pure returns (bytes32) {
        // The tssKey coord x is already collision resistant
        // if we are unsure about it, we can hash the coord and parity bit
        return bytes32(signature.xCoord);
    }

    // Register/Revoke TSS keys
    function _updateTssKeys(bytes32 messageHash, TssKey[] memory revokeKeys, TssKey[] memory registerKeys) private {
        // We don't perform any arithmetic operation, except iterate a loop
        unchecked {
            // Revoke tss keys
            for (uint256 i=0; i < revokeKeys.length; i++) {
                TssKey memory revokedKey = revokeKeys[i];

                // Read shard from storage
                bytes32 shardId = _tssKeyToShardId(revokedKey);
                ShardInfo storage shard = _shards[shardId];

                // Check if the shard exists and is active
                require(shard.nonce > 0, "shard doesn't exists, cannot revoke key");
                require((shard.status & SHARD_ACTIVE) > 0, "cannot revoke a shard key already revoked");

                // Check y-parity
                {
                    uint8 yParity = (shard.status & SHARD_Y_PARITY) > 0 ? 1 : 0;
                    require(yParity == revokedKey.yParity, "invalid y parity bit, cannot revoke key");
                }

                // Disable KEY_FLAG_ACTIVE bitflag
                shard.status = shard.status & (~SHARD_ACTIVE); // Disable active flag
            }

            // Register or enable tss key (old keys keep the same previous nonce)
            for (uint256 i=0; i < registerKeys.length; i++) {
                // Validate y-parity bit
                TssKey memory newKey = registerKeys[i];

                // Read shard from storage
                bytes32 shardId = _tssKeyToShardId(newKey);
                ShardInfo storage shard = _shards[shardId];
                uint8 status = shard.status;
                uint32 nonce = shard.nonce;

                // Check if the shard is not active
                require((status & SHARD_ACTIVE) == 0, "already active, cannot register again");

                // Check y-parity
                uint8 yParity = newKey.yParity;
                require(yParity == (yParity & 3), "y parity bit must be 0 or 1, cannot register shard");
                
                // If the shard exists, the y-parity must match the original one
                if (nonce > 0) {
                    uint8 actualYParity = (status & SHARD_Y_PARITY) > 0 ? 1 : 0;
                    require(actualYParity == yParity, "the provided y-parity doesn't match the existing y-parity, cannot register shard");
                }
                
                // enable SHARD_Y_PARITY bitflag
                if (yParity > 0) {
                    status |= SHARD_Y_PARITY;
                }

                // enable SHARD_ACTIVE bitflag
                status |= SHARD_ACTIVE;

                // Save status in the storage
                shard.status = status;

                // if shard nonce is zero, set it to 1
                if (nonce == 0) {
                    shard.nonce = 1;
                }
            }
        }
    }
    // Register/Revoke TSS keys using sudo account
    function _sudoUpdateTSSKeys(TssKey[] memory revokeKeys, TssKey[] memory registerKeys) external {
        require(msg.sender == _owner, "not autorized");
        _updateTssKeys(0, revokeKeys, registerKeys);
    }

    // Raw register/Revoke TSS keys using sudo account
    function rawSudoUpdateTSSKeys(uint8[] memory revokeKeysYParity, uint256[] memory revokeKeysXCoord, uint8[] memory registerKeysYParity, uint256[] memory registerKeysXCoord) external {
        require(msg.sender == _owner, "not autorized");
        TssKey[] memory revokeKeys = new TssKey[](revokeKeysYParity.length);
        TssKey[] memory registerKeys = new TssKey[](registerKeysYParity.length);

        for (uint i = 0; i < revokeKeys.length; i++) {
            revokeKeys[i] = TssKey(revokeKeysYParity[i], revokeKeysXCoord[i]);
        }

        for (uint i = 0; i < registerKeys.length; i++) {
            registerKeys[i] = TssKey(registerKeysYParity[i], registerKeysXCoord[i]);
        }

        _updateTssKeys(0, revokeKeys, registerKeys);
    }

    // Register/Revoke TSS keys using shard TSS signature
    function updateTSSKeys(Signature memory signature, UpdateShardsMessage memory message) external {
        bytes32 messageHash = getUpdateShardsMessageTypedHash(message);
        _processSignature(signature, messageHash, message.nonce);

        // Register shards pubkeys
        _updateTssKeys(messageHash, message.register, message.revoke);
    }

    // Forward GMP message
    function _execute(bytes32 payloadHash, GmpPayload memory message) private returns (uint8 status, bytes32 result) {
        // Verify if this GMP message was already executed
        GmpMessageInfo storage gmpInfo = _messages[payloadHash];
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
            // Using low-level assembly because the GMP is considered executed regardless
			// if the recipient contract reverts or not.

            let ptr := add(data, 32)
            let size := mload(data)
            // returns 1 if the call succeed, and 0 if it reverted
            success := call(
                gasLimit, // call gas limit (passing all the gas available)
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
        emit GmpExecuted(payloadHash, message.source, message.dest, status, result);
    }

    // Send GMP message using sudo account
    function sudoExecute(GmpPayload memory message) external returns (uint8 status, bytes32 result) {
        require(msg.sender == _owner, "not autorized");
        bytes32 payloadHash = _getGmpPayloadHash(message);
        (status, result) = _execute(payloadHash, message);
    }

    // Send GMP message using sudo account
    function rawSudoExecute(
        bytes32 source,      // Pubkey/Address of who sends the GMP message
        uint128 srcNetwork,  // Source chain identifier (it's the EIP-155 chain_id for ethereum networks)
        address dest,        // Destination/Recipient contract address
        uint128 destNetwork, // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
        uint256 gasLimit,    // Gas limit of the GMP call
        uint256 salt,        // Message salt, useful for sending two messages with same content
        bytes memory data    // Message data with no specified format
    ) external returns (uint8 status, bytes32 result) {
        require(msg.sender == _owner, "not authorized");

        // Create a GmpPayload struct instance from the provided arguments
        GmpPayload memory message = GmpPayload({
            source: source,
            srcNetwork: srcNetwork,
            dest: dest,
            destNetwork: destNetwork,
            gasLimit: gasLimit,
            salt: salt,
            data: data
        });

        bytes32 payloadHash = _getGmpPayloadHash(message);
        (status, result) = _execute(payloadHash, message);
    }

    // Execute GMP message using shard TSS signature
    function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result) {
        (bytes32 messageHash, bytes32 payloadHash) = getGmpMessageTypedHash(message);
        _processSignature(signature, messageHash, message.nonce);

        // Execute GMP message
        (status, result) = _execute(payloadHash, message.payload);
    }

    // Raw Execute GMP message using shard TSS signature
    function rawExecute(
        uint256 signatureXCoord,    // affine x-coordinate from Signature
        uint256 signatureE,         // 'e' component from Signature
        uint256 signatureS,         // 's' component from Signature
        uint32 messageNonce,        // 'nonce' from GmpMessage
        bytes32 messageSource,      // 'source' from GmpPayload within GmpMessage
        uint128 messageSrcNetwork,  // 'srcNetwork' from GmpPayload within GmpMessage
        address messageDest,        // 'dest' from GmpPayload within GmpMessage
        uint128 messageDestNetwork, // 'destNetwork' from GmpPayload within GmpMessage
        uint256 messageGasLimit,    // 'gasLimit' from GmpPayload within GmpMessage
        uint256 messageSalt,        // 'salt' from GmpPayload within GmpMessage
        bytes memory messageData    // 'data' from GmpPayload within GmpMessage
    ) external returns (uint8 status, bytes32 result) {
        // Recreate the Signature struct from the provided arguments
        Signature memory signature = Signature({
            xCoord: signatureXCoord,
            e: signatureE,
            s: signatureS
        });

        // Recreate the GmpPayload struct from the provided arguments
        GmpPayload memory payload = GmpPayload({
            source: messageSource,
            srcNetwork: messageSrcNetwork,
            dest: messageDest,
            destNetwork: messageDestNetwork,
            gasLimit: messageGasLimit,
            salt: messageSalt,
            data: messageData
        });

        // Recreate the GmpMessage struct using the recreated payload and provided nonce
        GmpMessage memory message = GmpMessage({
            nonce: messageNonce,
            payload: payload
        });

        (bytes32 messageHash, bytes32 payloadHash) = getGmpMessageTypedHash(message);
        _processSignature(signature, messageHash, message.nonce);

        // Execute GMP message
        (status, result) = _execute(payloadHash, payload);
    }
}