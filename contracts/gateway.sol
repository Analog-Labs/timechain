// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.7.0 <0.9.0;


struct Signature {
    uint8 parity;
    uint256 px;
    uint256 e;
    uint256 s;
}

//
struct TssKey {
    uint8 parity;    // public key y-coord parity (27 or 28)
    uint256 coordX;  // public key x-coord
}

// message for register tss keys
struct RegisterTssKeys {
    uint256 nonce;
    TssKey[] keys;
}

// message for revoke tss keys
struct RevokeTssKeys {
    uint256 nonce;
    TssKey[] keys;
}

// message for revoke tss keys
struct GMPMessage {
    uint128 nonce;
    uint128 networkId; // source network id
    bytes32 sender;    // sender public key
    address dest;      // dest contract
    bytes payload;     // message payload
}

// Shard info
struct ShardInfo {
    uint128 flags;
    uint128 nonce;
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
    function _getTssKeyHash(TssKey[] memory tssKeys)
        internal
        pure
        returns (bytes32)
    {
        return
            keccak256(
                abi.encode(
                    keccak256("TssKey(uint8 parity,uint256 coordX)"),
                    tssKeys
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
                    keccak256("TssKey(uint8 parity,bytes32 coordX)[]"),
                    tssKeys
                )
            );
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function getRegisterTssKeysTypedHash(RegisterTssKeys memory registerTssKeys)
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
                            keccak256("RegisterTssKeys(uint256 nonce,TssKey[] keys)"),
                            registerTssKeys.nonce,
                            _getTssKeyArrayHash(registerTssKeys.keys)
                        )
                    )
                )
            );
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function getRevokeTssKeysTypedHash(RevokeTssKeys memory revokeTssKeys)
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
                            keccak256("RevokeTssKeys(uint256 nonce,TssKey[] keys)"),
                            revokeTssKeys.nonce,
                            _getTssKeyArrayHash(revokeTssKeys.keys)
                        )
                    )
                )
            );
    }

    // computes the hash of the fully encoded EIP-712 message for the domain, which can be used to recover the signer
    function getGmpMessageTypedHash(GMPMessage memory message)
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
                            keccak256("GMPMessage(uint128 nonce,uint128 networkId,bytes32 sender,address dest,bytes payload)"),
                            message.nonce,
                            message.networkId,
                            message.sender,
                            message.dest,
                            keccak256(message.payload)
                        )
                    )
                )
            );
    }
}

contract Gateway is SigUtils {

    // Owner of this contract, who can execute sudo operations
    address _owner;

    // Shard data, maps the pubkey coordX (which is already collision resistant) to shard info.
    mapping (uint256 => ShardInfo) _shards;

    constructor() payable {
        _owner = msg.sender;
    }
    // secp256k1 group order
    uint256 constant public Q = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;

    // parity := public key y-coord parity (27 or 28)
    // px := public key x-coord
    // message := 32-byte message
    // e := schnorr signature challenge
    // s := schnorr signature
    function _verifyTssSignature(
        uint8 parity,
        uint256 px,
        bytes32 message,
        uint256 e,
        uint256 s
    ) private pure returns (bool) {
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

    // Verifies TSS signature, reverts if invalid
    function verifyTssSignature(Signature memory signature, bytes32 message) public pure returns (bool) {
        require(
            _verifyTssSignature(
                signature.parity,
                signature.px,
                message,
                signature.e,
                signature.s
            ),
            "invalid tss signature"
        );
        return true;
    }

    // Transfer the ownership of this contract to another account
    function transferOwnership(address newOwner) external returns (bool) {
        require(msg.sender == _owner, "not autorized");
        _owner = newOwner;
        return true;
    }

    // Internal register TSS keys
    function _registerTSSKeys(TssKey[] memory tssKeys) private {
        for (uint256 i=0; i < tssKeys.length; i++) {
            uint256 pubkey = tssKeys[i].coordX;
            ShardInfo storage shard = _shards[pubkey];
            require(shard.flags == 0, "shard already registered");
            shard.flags = 1;
            shard.nonce = 1;
        }
    }

    // Register TSS keys using sudo account
    function sudoRegisterTSSKeys(TssKey[] memory tssKeys) external {
        require(msg.sender == _owner, "not autorized");
        _registerTSSKeys(tssKeys);
    }

    // Register TSS keys using shard TSS signature
    function registerTSSKeys(Signature memory signature, TssKey[] memory tssKeys) external {
        ShardInfo storage signer = _shards[signature.px];
        require(signer.flags > 0, "shard no registered");
        RegisterTssKeys memory message = RegisterTssKeys({
            nonce: signer.nonce,
            keys: tssKeys
        });
        bytes32 messageHash = getRegisterTssKeysTypedHash(message);
        verifyTssSignature(signature, messageHash);

        // Increment shard nonce
        signer.nonce += 1;

        // Register shards pubkeys
        _registerTSSKeys(tssKeys);
    }

    // Internal revoke tss keys
    function _revokeTSSKeys(TssKey[] memory tssKeys) private {
        // Revoke shards
        for (uint256 i=0; i < tssKeys.length; i++) {
            uint256 pubkey = tssKeys[i].coordX;
            ShardInfo storage shard = _shards[pubkey];
            require(shard.flags > 0, "shard not registered, cannot revoke");
            delete _shards[pubkey];
        }
    }

    // Revoke TSS keys using sudo account
    function sudoRevokeTSSKeys(TssKey[] memory tssKeys) external {
        require(msg.sender == _owner, "not autorized");
        _registerTSSKeys(tssKeys);
    }

    // Revoke TSS keys using shard TSS signature
    function revokeTSSKeys(Signature memory signature, TssKey[] memory tssKeys) external {
        ShardInfo storage signer = _shards[signature.px];
        require(signer.flags > 0, "shard no registered");
        RevokeTssKeys memory message = RevokeTssKeys({
            nonce: signer.nonce,
            keys: tssKeys
        });
        bytes32 messageHash = getRevokeTssKeysTypedHash(message);
        verifyTssSignature(signature, messageHash);

        // Increment shard nonce
        signer.nonce += 1;

        // Revoke shards keys
        _revokeTSSKeys(tssKeys);
    }

    // Internal GMP execute
    function _execute(GMPMessage memory message) private returns (bool success) {
        // TODO: call an interface
        (success,) = message.dest.call(message.payload);
    }

    // Send GMP message using sudo account
    function sudoExecute(GMPMessage memory message) external returns (bool success) {
        require(msg.sender == _owner, "not autorized");
        success = _execute(message);
    }

    // Execute GMP message using shard TSS signature
    function execute(Signature memory signature, GMPMessage memory message) external returns (bool success) {
        ShardInfo storage signer = _shards[signature.px];
        require(signer.nonce == message.nonce, "invalid gmp message nonce");
        bytes32 messageHash = getGmpMessageTypedHash(message);
        verifyTssSignature(signature, messageHash);

        // Increment shard nonce
        signer.nonce += 1;

        // Execute GMP message
        success = _execute(message);
    }
}