use crate::GatewayMessage;

use super::{
    GmpMessage, GmpEvent,
    config::{Config, BlockNumberOf, AddressOf}
};
use polkadot_sdk::{
    sp_std::{fmt::{Debug, Display},  hash::Hash, marker::{Send, Sync}, convert::{AsRef, AsMut, TryFrom}, cmp::Ord, vec::Vec, ops::Range},
	sp_runtime::traits::{Member, MaybeDisplay, MaybeFromStr, MaybeSerializeDeserialize, MaybeHash, Hash as HashT, BlockNumber, HashOutput},
    sp_arithmetic::{
        FixedPointOperand,
        FixedPointNumber,
        traits::{AtLeast32BitUnsigned, Saturating, Bounded, CheckedSub}
    },
};
use scale_codec::{MaxEncodedLen, Codec};
use scale_info::TypeInfo;


/// Minimal Chain representation that may be used from ``no_std`` environment.
pub trait ConnectorBackend: Send + Sync + 'static {
    type Config: Config;

	/// An error type when fetching or sending data.
    type Error: Send + Sync + Debug + Display + 'static;

    /// Sends a message using the test contract.
	fn send_message(&self, message: GatewayMessage) -> Result<(), Self::Error>;

    /// Messages sent from the target chain to the timechain.
	fn inbound_messages(&self, blocks: Range<BlockNumberOf<Self::Config>>) -> Result<Vec<GmpEvent>, Self::Error>;

    /// Messages sent from the timechain to the target chain.
	fn outbound_messages(&self, blocks: Range<BlockNumberOf<Self::Config>>) -> Result<Vec<GmpMessage>, Self::Error>;
}
