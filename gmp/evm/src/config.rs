use time_primitives::gmp::{GatewayOp, config::{PayloadT, MessageT, Config, HeaderT, Location, TssSignature, TssSigHash}};
use polkadot_sdk::{
    sp_std::{self, vec::Vec, convert::{AsMut, AsRef}, fmt::{Display, Formatter}},
    sp_core::{U256, H160, H256, KeccakHasher},
    sp_runtime::traits::Keccak256,
};
use scale_codec::{Encode, Decode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
pub struct Context {
    network_id: u16,
    gateway_address: H160,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
pub struct Header {
    hash: H256,
    parent_hash: H256,
    block_number: u64,
}

impl HeaderT for Header {
    type Number = <EvmConfig as Config>::BlockNumber;
    type Hash = <EvmConfig as Config>::Hash;

    fn hash(&self) -> Self::Hash {
        self.hash
    }

    fn parent_hash(&self) -> Self::Hash {
        self.parent_hash
    }

    fn set_parent_hash(&mut self, hash: Self::Hash) {
        self.parent_hash = hash;
    }

    fn number(&self) -> Self::Number {
        self.block_number
    }

    fn set_number(&mut self, number: Self::Number) {
        self.block_number = number;
    }
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Payload {
    origin: u16,
    target: u16,
    op: GatewayOp,
}

impl PayloadT<Context> for Payload {
    fn new(ctx: &Context, op: GatewayOp) -> Self {
        Self {
            origin: ctx.network_id,
            target: ctx.network_id,
            op,
        }
    }

    fn operation(&self) -> GatewayOp {
        self.op.clone()
    }

    fn location(&self) -> Location {
        Location {
            origin: Some(self.origin),
            destination: self.target,
        }
    }

    fn sighash(&self, ctx: Context) -> TssSigHash {
        TssSigHash([0; 32])
    }
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq, Hash)]
pub struct Message {
    origin: u16,
    target: u16,
    payload: Payload,
    signature: TssSignature,
}

impl MessageT<Context, Payload, H256> for Message {
    fn new(ctx: &Context, payload: Payload, signature: TssSignature) -> Self {
        Self {
            origin: ctx.network_id,
            target: ctx.network_id,
            payload,
            signature,
        }
    }

    fn batch_id(&self) -> H256 {
        H256::zero()
    }

    fn signature(&self) -> TssSignature {
        self.signature.clone()
    }

    fn location(&self) -> Location {
        self.payload.location()
    }

    fn operations(&self) -> impl Iterator<Item = &GatewayOp> + Send + Sync {
        sp_std::iter::once(&self.payload.op)
    }
}

pub enum EvmConfig {}

impl Config for EvmConfig {
    const SYMBOL: &'static str = "ETH";

    const DECIMALS: u8 = 18;

    type Context = Context;

    type Payload = Payload;

    type Message = Message;

    type BlockNumber = u64;

    type Address = H160;

    type Balance = U256;

    type Hash = H256;

    type Hasher = Keccak256;

    type Header = Header;
}
