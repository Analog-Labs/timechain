use crate::{
	AccountId, AllianceMotion, Assets, Authorship, Balances, Call, Hash, NegativeImbalance, Runtime,
};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::{Balanced, CreditOf},
		Currency, OnUnbalanced,
	},
};

/// A `HandleCredit` implementation that naively transfers the fees to the block author.
/// Will drop and burn the assets in case the transfer fails.
pub struct CreditToBlockAuthor;
impl HandleCredit<AccountId, Assets> for CreditToBlockAuthor {
	fn handle_credit(credit: CreditOf<AccountId, Assets>) {
		if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
			// Drop the result which will trigger the `OnDrop` of the imbalance in case of error.
			let _ = Assets::resolve(&author, credit);
		}
	}
}