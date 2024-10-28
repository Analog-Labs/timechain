use polkadot_sdk::{
	frame_support::dispatch::Parameter,
    sp_std::{fmt::Debug, hash::Hash, marker::{Send, Sync}, convert::{AsRef, AsMut, TryFrom}, cmp::Ord},
    sp_core::U256,
	sp_runtime::traits::{Member, MaybeDisplay, MaybeFromStr, MaybeSerializeDeserialize, MaybeHash, Hash as HashT, BlockNumber, HashOutput},
    sp_arithmetic::{
        FixedPointOperand,
        FixedPointNumber,
        traits::{AtLeast32BitUnsigned, AtLeast32Bit, Saturating, Bounded, CheckedSub}
    },
};
use scale_codec::{MaxEncodedLen, Codec, Encode, Decode};
use scale_info::TypeInfo;
use crate::{GatewayMessage, GmpMessage, GatewayOp, NetworkId};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[repr(transparent)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct TssPublicKey(pub [u8; 33]);

#[repr(transparent)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct TssSignature(pub [u8; 64]);

#[repr(transparent)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct TssSigHash(pub [u8; 32]);

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct Location {
    pub origin: Option<NetworkId>,
    pub destination: NetworkId,
}

pub trait PayloadT<CTX> {
    fn new(ctx: &CTX, op: GatewayOp) -> Self;
    fn operation(&self) -> GatewayOp;
    fn location(&self) -> Location;
    fn sighash(&self, ctx: CTX) -> TssSigHash;
}

pub trait MessageT<CTX, PAYLOAD, ID> {
    fn new(ctx: &CTX, payload: PAYLOAD, signature: TssSignature) -> Self;
    fn batch_id(&self) -> ID;
    fn signature(&self) -> TssSignature;
    fn location(&self) -> Location;
    fn operations(&self) -> impl Iterator<Item = &GatewayOp> + Send + Sync;
}

/// Something which fulfills the abstract idea of a Block Header. It has types for a `Number`,
/// a `Hash` and a `Hashing`. It provides access to an `extrinsics_root`, `state_root` and
/// `parent_hash`, as well as a `digest` and a block `number`.
///
/// You can also create a `new` one from those fields.
pub trait HeaderT: Send + Sync + Clone + Eq + Debug + Codec + TypeInfo + 'static {
    /// Header number.
    type Number: BlockNumber;
    
    /// Header hash type
	type Hash: HashOutput;

    /// Returns the hash of the header.
	fn hash(&self) -> Self::Hash;

    /// Returns a reference to the parent hash.
	fn parent_hash(&self) -> Self::Hash;

	/// Sets the parent hash.
	fn set_parent_hash(&mut self, hash: Self::Hash);

    /// Returns a reference to the header number.
	fn number(&self) -> Self::Number;

    /// Sets the header number.
	fn set_number(&mut self, number: Self::Number);
}

/// Minimal Chain representation that may be used from ``no_std`` environment.
pub trait Config: Send + Sync + 'static {
    const SYMBOL: &'static str;
    const DECIMALS: u8;

    /// Message context type.
    type Context: Parameter + Member + Codec + Eq + TypeInfo + 'static;

    /// Message payload unsigned.
    type Payload: PayloadT<Self::Context> + Parameter + Member + Hash + 'static;

    /// Message signed.
    type Message: MessageT<Self::Context, Self::Payload, Self::Hash> + Parameter + Member + Hash + 'static;

    /// A type that fulfills the abstract idea of what a block number is.
	// Constraints come from the associated Number type of `sp_runtime::traits::Header`
	type BlockNumber: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Hash
        + Copy
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + MaybeFromStr
        + Default
        + Saturating
        + MaxEncodedLen;

    /// The type used to identify an account in the chain.
    type Address: Parameter + Member + MaxEncodedLen + MaybeDisplay + MaybeFromStr + Ord + Debug + Clone;
    
    /// Balance of an account in native tokens.
	///
	/// The chain may support multiple tokens, but this particular type is for token that is used
	/// to pay for transaction fees.
    type Balance: Parameter
        + Member
        // + AtLeast32Bit
        // + AtLeast32BitUnsigned
        // + FixedPointOperand
        // + Saturating
        + MaxEncodedLen
        + MaybeDisplay
        + MaybeFromStr
        + TryFrom<U256>
        + Copy;
    
    /// A type that fulfills the abstract idea of what a hash is.
	type Hash: HashOutput;

    // A type that fulfills the abstract idea of what a hasher (a type
	/// that produces hashes) is.
	type Hasher: HashT<Output = Self::Hash>;

    /// A type that fulfills the abstract idea of what a Substrate header is.
	// See here for more info:
	// https://crates.parity.io/sp_runtime/traits/trait.Header.html
	type Header: Parameter
        + HeaderT<Number = Self::BlockNumber, Hash = Self::Hash>
        + MaybeSerializeDeserialize;
}

/// Context used to parse and build GMP messages.
pub type ContextOf<C> = <C as Config>::Context;

/// Payload used to build GMP messages.
pub type PayloadOf<C> = <C as Config>::Payload;

/// The message TSS signed to send to the destination chain.
pub type MessageOf<C> = <C as Config>::Message;

/// Block number used by the chain.
pub type AddressOf<C> = <C as Config>::Address;

/// Block number used by the chain.
pub type BlockNumberOf<C> = <C as Config>::BlockNumber;

/// Hash type used by the chain.
pub type HashOf<C> = <C as Config>::Hash;

/// Hasher type used by the chain.
pub type HasherOf<C> = <C as Config>::Hasher;

/// Balance type used by the chain.
pub type BalanceOf<C> = <C as Config>::Balance;

/// Header type used by the chain.
pub type HeaderOf<C> = <C as Config>::Header;
