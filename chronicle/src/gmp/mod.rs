use alloy_sol_types::{sol, SolCall, Eip712Domain, SolStruct};
use alloy_primitives::{U256, Address};
use sha3::{Keccak256, Digest};

const EIP712_NAME: &str = "Analog Gateway Contract";
const EIP712_VERSION: &str = "0.1.0";

pub type Eip712Bytes = [u8; 66];
#[allow(dead_code)]
pub type Eip712Hash = [u8; 32];

pub fn eip712_domain_separator(chain_id: u64, gateway_contract: Address) -> Eip712Domain {
	Eip712Domain {
		name: Some(EIP712_NAME.into()),
		version: Some(EIP712_VERSION.into()),
		chain_id: Some(U256::from(chain_id)),
		verifying_contract: Some(gateway_contract),
		salt: None,
	}
}

sol! {
    #[derive(Debug, Default, PartialEq, Eq)]
    struct TssKey {
        uint8 yParity;
        uint256 xCoord;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct UpdateKeysMessage {
        TssKey[] revoke;
        TssKey[] register;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct GmpMessage {
        bytes32 source;
        uint128 srcNetwork;
        address dest;
        uint128 destNetwork;
        uint256 gasLimit;
        uint256 salt;
        bytes data;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Signature {
        uint256 xCoord;
        uint256 e;
        uint256 s;
    }

    interface IGateway {
        event GmpExecuted(
            bytes32 indexed id,
            bytes32 indexed source,
            address indexed dest,
            uint256 status,
            bytes32 result
        );

        event KeySetChanged(
            bytes32 indexed id,
            TssKey[] revoked,
            TssKey[] registered
        );

        function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result);
        function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
    }
}

impl From<[u8;33]> for TssKey {
    fn from(bytes: [u8;33]) -> Self {
        Self {
            yParity: bytes[0],
            xCoord: U256::from_be_slice(&bytes[1..])
        }
    }
}

/// Extends the [`SolStruct`] to accept the chain_id and gateway contract address as parameter
pub trait Eip712Ext {
	fn to_eip712_bytes_with_domain(&self, domain_separator: &Eip712Domain) -> Eip712Bytes;
    fn to_eip712_bytes(&self, chain_id: u64, gateway_contract: Address) -> Eip712Bytes {
		let domain_separator = eip712_domain_separator(chain_id, gateway_contract);
        Eip712Ext::to_eip712_bytes_with_domain(self, &domain_separator)
	}
    fn to_eip712_typed_hash(&self, chain_id: u64, gateway_contract: Address) -> Eip712Hash {
		let bytes = Eip712Ext::to_eip712_bytes(self, chain_id, gateway_contract);
		let mut hasher = Keccak256::new();
		hasher.update(bytes.as_ref());
		hasher.finalize().into()
	}
}

// Implements the [`Eip712Ext`] for all [`SolStruct`]
impl <T> Eip712Ext for T where T: SolStruct {
	fn to_eip712_bytes_with_domain(&self, domain_separator: &Eip712Domain) -> Eip712Bytes {
        let mut digest_input = [0u8; 2 + 32 + 32];
        digest_input[0] = 0x19;
        digest_input[1] = 0x01;
        digest_input[2..34].copy_from_slice(&domain_separator.hash_struct()[..]);
        digest_input[34..66].copy_from_slice(&self.eip712_hash_struct()[..]);
        digest_input
    }
}

pub trait SignableMessage: Eip712Ext {
    type Method: SolCall;
    fn into_call(self, signature: Signature) -> Self::Method;
}

impl SignableMessage for GmpMessage {
    type Method = IGateway::executeCall;

    fn into_call(self, signature: Signature) -> Self::Method {
        IGateway::executeCall { signature, message: self }
    }
}

impl SignableMessage for UpdateKeysMessage {
    type Method = IGateway::updateKeysCall;

    fn into_call(self, signature: Signature) -> Self::Method {
        IGateway::updateKeysCall { signature, message: self }
    }
}
