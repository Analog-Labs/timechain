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
	sp_runtime::traits::{AccountIdConversion, CheckedAdd, Saturating, Zero},
};

pub use pallet::*;
pub use types::{NetworkData, NetworkDetails};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type NetworkIdOf<T> = <T as Config>::NetworkId;
pub type NegativeImbalanceOf<T> =
	<<T as Config>::Currency as Currency<AccountIdOf<T>>>::NegativeImbalance;

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

// TODO change to AccountId20
pub type NetworkDataOf<T> = NetworkData<AccountIdOf<T>>;
pub type NetworkDetailsOf<T> = NetworkDetails<BalanceOf<T>, AccountIdOf<T>>;
pub type BeneficiaryOf<T> = <T as Config>::Beneficiary;

/// Teleport handlers.
pub trait AssetTeleporter<T: Config> {
	/// Attempt to register `network_id` with `data` for assets teleportation.
	/// This we need to have guarantee that network is active, i.e. actually has some
	/// chronicle shards working with it.
	fn handle_register(network_id: NetworkIdOf<T>, data: &mut NetworkDataOf<T>) -> DispatchResult;

	/// Teleport `amount` of tokens to `network_id` for `beneficiary` account.
	/// This method is called only after the asset get successfully locked in this pallet.
	fn handle_teleport(
		source: &T::AccountId,
		network_id: NetworkIdOf<T>,
		data: &mut NetworkDataOf<T>,
		beneficiary: &BeneficiaryOf<T>,
		amount: BalanceOf<T>,
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
		fn force_update_network() -> Weight;
		fn register_network() -> Weight;
	}

	impl WeightInfo for () {
		fn teleport_keep_alive() -> Weight {
			Weight::default()
		}
		fn force_teleport() -> Weight {
			Weight::default()
		}
		fn force_update_network() -> Weight {
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
		/// Account for holding the teleported assets.
		/// This is advised to be set to inaccesible Account ID, e.g. the one provided by `Self::account_id()`.
		#[pallet::constant]
		type BridgePot: Get<Self::AccountId>;

		/// The bridge balance.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Handler for the unbalanced decrease when teleport fees are paid.
		type FeeDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// Handler responsible to teleport the assets.
		type Teleporter: AssetTeleporter<Self>;

		/// Network unique identifier
		type NetworkId: Parameter + MaxEncodedLen;

		/// Identifier the account getting the teleported funds on the target chain,
		/// currently this should be set to AccountId20
		type Beneficiary: Parameter + MaxEncodedLen;

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
		/// Failed to lock teleport amount.
		CannotReserveFunds,
		/// The teleport amount cannot be zero.
		AmountZero,
		/// Attempt to use a network_id already in use.
		NetworkAlreadyExists,
		/// Network data nonce onverflow.
		NetworkNonceOverflow,
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
			Self::do_teleport_out(
				source,
				network,
				beneficiary,
				amount,
				ExistenceRequirement::KeepAlive,
			)
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
			Self::do_teleport_out(
				source,
				network,
				beneficiary,
				amount,
				ExistenceRequirement::KeepAlive,
			)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_network())]
		pub fn register_network(
			origin: OriginFor<T>,
			network: T::NetworkId,
			base_fee: BalanceOf<T>,
			data: NetworkDataOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_register_network(network, base_fee, data)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::register_network())]
		pub fn force_update_network(
			origin: OriginFor<T>,
			network: T::NetworkId,
			active: bool,
			maybe_data: Option<NetworkDataOf<T>>,
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
			const BRIDGE_PALLET_ID: PalletId = PalletId(*b"py/bridg");

			BRIDGE_PALLET_ID.into_account_truncating()
		}

		/// Return the amount of money reserved by the bridge account.
		/// The existential deposit is not part of the reserve so the bridge account never gets deleted.
		pub fn reserved() -> BalanceOf<T> {
			T::Currency::free_balance(&T::BridgePot::get())
				// Must never be less than 0 but better be safe.
				.saturating_sub(T::Currency::minimum_balance())
		}

		pub fn do_register_network(
			network: T::NetworkId,
			base_fee: BalanceOf<T>,
			mut data: NetworkDataOf<T>,
		) -> DispatchResult {
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
		/// Perform the asset teleportation to other network
		/// emits `Event::Teleported`.
		pub fn do_teleport_out(
			sender: T::AccountId,
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

				// Withdraw `total` from `sender`
				let reason = WithdrawReasons::TRANSFER | WithdrawReasons::FEE;
				let imbalance = T::Currency::withdraw(&sender, total, reason, liveness)?;

				// If `network_details.teleport_base_fee` is greater than zero, pay the fee to destination
				let imbalance = if !details.teleport_base_fee.is_zero() {
					let (fee, reserve) = imbalance.split(details.teleport_base_fee);
					T::FeeDestination::on_unbalanced(fee);
					reserve
				} else {
					imbalance
				};

				let reserve = imbalance.peek();

				// Lock: put `amount` into the bridge pot.
				details.total_locked = details.total_locked.saturating_add(reserve);
				let locked = T::Currency::deposit_creating(&T::BridgePot::get(), reserve);
				// This happens only when transferred < ED and bridge account is empty
				ensure!(!locked.peek().is_zero(), Error::<T>::CannotReserveFunds);
				drop(imbalance.offset(locked));

				// Perform the teleport
				T::Teleporter::handle_teleport(
					&sender,
					network_id.clone(),
					&mut details.data,
					&beneficiary,
					amount,
				)
			})?;

			// Emit `Teleported` event.
			Self::deposit_event(Event::Teleported { account: sender, amount });

			Ok(())
		}

		/// Process assets teleported from other network
		pub fn do_teleport_in(recipient: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
			T::Currency::transfer(
				&T::BridgePot::get(),
				recipient,
				amount,
				ExistenceRequirement::KeepAlive,
			)
		}
	}
}
