#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;

extern crate alloc;

use scale_codec::MaxEncodedLen;

use polkadot_sdk::{
	frame_support::{
		self,
		dispatch::DispatchResult,
		traits::{Currency, Get, Imbalance, OnUnbalanced, WithdrawReasons},
		weights::Weight,
		PalletId,
	},
	frame_system::{self},
	sp_runtime::traits::{AccountIdConversion, CheckedAdd, Saturating, StaticLookup, Zero},
};

pub use pallet::*;
pub use types::{ExistenceRequirement, NetworkChannel, NetworkDetails};

pub type NetworkIdOf<T> = <T as Config>::NetworkId;

pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NetworkDataOf<T> = <T as Config>::NetworkData;
pub type NetworkDetailsOf<T> = NetworkDetails<BalanceOf<T>, NetworkDataOf<T>>;

/// Teleport handlers.
pub trait AssetTeleporter<NetworkId, NetworkData, Beneficiary, Balance> {
	/// Attempt to register `network_id` with `data`.
	fn handle_register(network_id: NetworkId, data: &mut NetworkData) -> DispatchResult;

	/// Teleport `amount` of tokens to `network_id` for `beneficiary` account.
	/// This method is called only after the asset get successfully locked in this pallet.
	fn handle_teleport(
		network_id: NetworkId,
		details: &mut NetworkData,
		beneficiary: Beneficiary,
		amount: Balance,
	) -> DispatchResult;
}

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, ReservableCurrency},
	};

	use frame_system::pallet_prelude::{ensure_root, ensure_signed, OriginFor};

	pub trait WeightInfo {
		fn teleport_keep_alive() -> Weight;
		fn force_teleport() -> Weight;
		fn register_network() -> Weight;
	}

	impl WeightInfo for () {
		fn teleport_keep_alive() -> Weight {
			Weight::default()
		}
		fn force_teleport() -> Weight {
			Weight::default()
		}
		fn register_network() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// The basic amount of funds that will be charged for cover the teleport costs.
		#[pallet::constant]
		type TeleportBaseFee: Get<BalanceOf<Self>>;

		/// The bridge pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The bridge balance.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Handler for the unbalanced decrease when teleport fees are paid.
		type FeeDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// Handler responsible to teleport the assets.
		type Teleporter: AssetTeleporter<
			Self::NetworkId,
			Self::NetworkData,
			Self::Beneficiary,
			BalanceOf<Self>,
		>;

		/// Network unique identifier
		type NetworkId: Parameter + MaxEncodedLen;

		/// Network unique identifier
		type NetworkData: Parameter + MaxEncodedLen;

		/// Type parameter used to identify the beneficiaries eligible to receive treasury spends.
		type Beneficiary: Parameter + MaxEncodedLen;

		/// Converting trait to take a source type and convert to [`Self::Beneficiary`].
		type BeneficiaryLookup: StaticLookup<Target = Self::Beneficiary>;

		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		// type MoveClaimOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type WeightInfo: WeightInfo;
	}

	/// Get network for member
	#[pallet::storage]
	pub type Network<T> =
		StorageMap<_, Blake2_128Concat, NetworkIdOf<T>, NetworkDetailsOf<T>, OptionQuery>;

	/// Define events emitted by the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// We have ended a spend period and will now allocate funds.
		Teleported { account: T::AccountId, amount: BalanceOf<T> },
		/// We have ended a spend period and will now allocate funds.
		BridgeStatusChanged { network: T::NetworkId, open: bool },
	}

	///  Define possible errors that can occur during pallet operations.
	#[pallet::error]
	pub enum Error<T> {
		/// Network Not Found or disabled.
		NetworkDisabled,
		/// No sufficient funds for pay for the teleportation.
		InsufficientFunds,
		/// Failed to lock funds paid by the account, should never happen.
		CannotReserveFunds,
		/// The teleport amount cannot be zero.
		AmountZero,
		/// Attempt to use a network_id already in use.
		NetworkAlreadyExists,
	}

	/// Exposes callable functions to interact with the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::teleport_keep_alive())]
		pub fn teleport_keep_alive(
			origin: OriginFor<T>,
			network: T::NetworkId,
			beneficiary: T::Beneficiary,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			Self::do_teleport(source, network, beneficiary, amount, ExistenceRequirement::KeepAlive)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::force_teleport())]
		pub fn force_teleport(
			origin: OriginFor<T>,
			source: T::AccountId,
			network: T::NetworkId,
			beneficiary: T::Beneficiary,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_teleport(source, network, beneficiary, amount, ExistenceRequirement::KeepAlive)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_network())]
		pub fn register_network(
			origin: OriginFor<T>,
			network: T::NetworkId,
			base_fee: BalanceOf<T>,
			mut data: T::NetworkData,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(!Network::<T>::contains_key(&network), Error::<T>::NetworkAlreadyExists);
			T::Teleporter::handle_register(network.clone(), &mut data)?;
			let details = NetworkDetails {
				active: true,
				teleport_base_fee: base_fee,
				total_locked: BalanceOf::<T>::zero(),
				data,
			};
			Network::<T>::insert(network.clone(), details);

			// Emit `BridgeStatusChanged` event.
			Self::deposit_event(Event::BridgeStatusChanged { network, open: true });

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::register_network())]
		pub fn force_update_network(
			origin: OriginFor<T>,
			network: T::NetworkId,
			active: bool,
			maybe_data: Option<T::NetworkData>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let network_ref = &network;
			Network::<T>::try_mutate_exists(network_ref, move |maybe_network| -> DispatchResult {
				// Check if the network exists.
				let details = maybe_network.as_mut().ok_or(Error::<T>::NetworkDisabled)?;

				if details.active != active {
					// Emit `BridgeStatusChanged` event.
					Self::deposit_event(Event::BridgeStatusChanged {
						network: network_ref.clone(),
						open: active,
					});
				}

				details.active = active;
				if let Some(data) = maybe_data {
					details.data = data;
				}
				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of the bridge pot.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache the
		/// value and only call this once.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		/// Return the amount of money reserved by the bridge account.
		/// The existential deposit is not part of the reserve so the bridge account never gets deleted.
		pub fn reserved() -> BalanceOf<T> {
			T::Currency::free_balance(&Self::account_id())
				// Must never be less than 0 but better be safe.
				.saturating_sub(T::Currency::minimum_balance())
		}

		/// Perform the asset teleportation
		/// emits `Event::Teleported`.
		fn do_teleport(
			source: T::AccountId,
			network_id: T::NetworkId,
			beneficiary: T::Beneficiary,
			amount: BalanceOf<T>,
			liveness: ExistenceRequirement,
		) -> DispatchResult {
			ensure!(!amount.is_zero(), Error::<T>::AmountZero);

			Network::<T>::try_mutate_exists(&network_id, |maybe_network| -> DispatchResult {
				// Check if the network exists.
				let details = maybe_network.as_mut().ok_or(Error::<T>::NetworkDisabled)?;
				ensure!(details.active, Error::<T>::NetworkDisabled);

				// total = amount + network_details.teleport_base_fee
				let total = amount
					.checked_add(&details.teleport_base_fee)
					.ok_or(Error::<T>::InsufficientFunds)?;

				// Withdraw `total` from `source`
				let reason = WithdrawReasons::TRANSFER | WithdrawReasons::FEE;
				let imbalance = T::Currency::withdraw(&source, total, reason, liveness)?;

				// If `network_details.teleport_base_fee` is greater than zero, pay the fee to destination
				let reserve = if !details.teleport_base_fee.is_zero() {
					let (fee, reserve) = imbalance.split(details.teleport_base_fee);
					T::FeeDestination::on_unbalanced(fee);
					reserve
				} else {
					imbalance
				};

				// Lock `amount` into the bridge pot.
				details.total_locked = details.total_locked.saturating_add(reserve.peek());
				let dest = Self::account_id();
				if let Err(problem) = T::Currency::resolve_into_existing(&dest, reserve) {
					// Must never be an error, but better to be safe.
					frame_support::print("Inconsistent state - couldn't reserve imbalance for funds teleported by source");
					drop(problem);
					return Err(Error::<T>::CannotReserveFunds.into());
				}

				// Perform the teleport
				T::Teleporter::handle_teleport(
					network_id.clone(),
					&mut details.data,
					beneficiary,
					amount,
				)
			})?;

			// Emit `Teleported` event.
			Self::deposit_event(Event::Teleported { account: source, amount });

			Ok(())
		}
	}
}
