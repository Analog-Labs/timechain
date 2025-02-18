#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]

//! This pallet manages members' registration, heartbeat functionality, and
//! member management within a decentralized network.
//!
//! This flowchart represents the control flow and interactions of callable
//! functions (`register_member`, `send_heartbeat`, `unregister_member`). It shows
//! the decision points, data operations, and event emissions along with error
//! handling where applicable.
//!
//!
#![doc = simple_mermaid::mermaid!("../docs/member_calls.mmd")]
//!
//! This flowchart illustrates the decision-making and steps taken within the
//! `on_initialize` function, highlighting the main actions and checks performed
//! during the process.
//!
#![doc = simple_mermaid::mermaid!("../docs/member_hooks.mmd")]
//!

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{frame_support, frame_system, sp_runtime, sp_std};

	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, ExistenceRequirement, ReservableCurrency};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{IdentifyAccount, Zero};
	use sp_std::vec::Vec;

	use polkadot_sdk::pallet_balances;

	use time_primitives::{
		AccountId, Balance, ElectionsInterface, MembersInterface, NetworkId, PeerId, PublicKey,
		ShardsInterface,
	};

	pub trait WeightInfo {
		fn register_member() -> Weight;
		fn send_heartbeat() -> Weight;
		fn unregister_member() -> Weight;
		fn timeout_heartbeats(n: u32) -> Weight;
	}

	impl WeightInfo for () {
		fn register_member() -> Weight {
			Weight::default()
		}
		fn send_heartbeat() -> Weight {
			Weight::default()
		}
		fn unregister_member() -> Weight {
			Weight::default()
		}
		fn timeout_heartbeats(_: u32) -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	pub type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId>
		+ pallet_balances::Config<Balance = Balance>
	{
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
		type Elections: ElectionsInterface;
		/// Minimum stake to register member
		#[pallet::constant]
		type MinStake: Get<BalanceOf<Self>>;
		#[pallet::constant]
		type HeartbeatTimeout: Get<BlockNumberFor<Self>>;
		#[pallet::constant]
		type MaxTimeoutsPerBlock: Get<u32>;
	}

	/// Get network for member
	#[pallet::storage]
	pub type MemberNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, NetworkId, OptionQuery>;

	/// Get PeerId for member
	#[pallet::storage]
	pub type MemberPeerId<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, PeerId, OptionQuery>;

	/// Get PublicKey for member
	#[pallet::storage]
	pub type MemberPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, PublicKey, OptionQuery>;

	/// Get status of member
	#[pallet::storage]
	pub type MemberOnline<T: Config> = StorageMap<_, Blake2_128Concat, AccountId, (), OptionQuery>;

	/// Get whether member submitted heartbeat within last period
	#[pallet::storage]
	pub type Heartbeat<T: Config> = StorageMap<_, Blake2_128Concat, AccountId, (), OptionQuery>;

	/// Set of members that have not submitted a heartbeat within last period
	#[pallet::storage]
	pub type TimedOut<T: Config> = StorageValue<_, Vec<AccountId>, ValueQuery>;

	/// Get stake for member
	#[pallet::storage]
	pub type MemberStake<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, BalanceOf<T>, ValueQuery>;

	/// Get account that staked.
	#[pallet::storage]
	pub type MemberStaker<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, AccountId, OptionQuery>;

	/// Get if member is electable.
	#[pallet::storage]
	pub type MemberRegistered<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, (), OptionQuery>;

	/// Define events emitted by the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [`Event::RegisteredMember`] shard member registration.
		RegisteredMember(AccountId, NetworkId, PeerId),

		/// [`Event::HeartbeatReceived`] heartbeat reception event.
		HeartbeatReceived(AccountId),

		/// [`Event::MemberOnline`]  member online status changes
		MemberOnline(AccountId),

		/// [`Event::MemberOffline`] member offline status changes
		MemberOffline(AccountId),

		/// [`Event::UnRegisteredMember`] member unregistration event.
		UnRegisteredMember(AccountId, NetworkId),
	}

	///  Define possible errors that can occur during pallet operations.
	#[pallet::error]
	pub enum Error<T> {
		/// Not a member.
		NotMember,
		/// Bond below min stake.
		BondBelowMinStake,
		/// Staked below transfer amount.
		StakedBelowTransferAmount,
		/// Member not registered.
		NotRegistered,
		/// Not staker.
		NotStaker,
		/// Already registered
		StillStaked,
	}

	/// Implements hooks for pallet initialization and block processing.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			log::info!("on_initialize begin");
			let weight = if (n % T::HeartbeatTimeout::get()).is_zero() {
				Self::timeout_heartbeats()
			} else {
				Weight::default()
			};
			log::info!("on_initialize end");
			weight
		}
	}

	/// Exposes callable functions to interact with the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// `register_member`: Registers a member with specified network ID, public key, peer ID, and bond (staking amount).
		/// # Flow
		///	1. Receives `origin` (caller's account), `network` (NetworkId), `public_key` (PublicKey), `peer_id` (PeerId), `bond` (Balance to stake).
		///	2. Ensures the `origin` is signed (authenticated).
		///	3. Validates the `public_key` against the `origin` account.
		///	4. Checks if the member is already registered and unregisters them if necessary.
		///	5. Ensures the `bond` is at least equal to `MinStake::get()`.
		///	6. Reserves the `bond` amount using [`pallet_balances::Pallet::<T>::reserve`].
		///	7. Inserts member data into respective storage maps ([`MemberNetwork::<T>`], [`MemberPublicKey::<T>`], [`MemberPeerId::<T>`], [`MemberStake::<T>`], [`Heartbeat::<T>`]).
		///	8. Marks the member as online ([`MemberOnline::<T>`]).
		///	9. Emits [`Event::RegisteredMember`].
		///	10. Calls `Self::member_online` to notify the network election system.
		///	11. Returns `Ok(())` if successful.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_member())]
		pub fn register_member(
			origin: OriginFor<T>,
			network: NetworkId,
			public_key: PublicKey,
			peer_id: PeerId,
			bond: BalanceOf<T>,
		) -> DispatchResult {
			let staker = ensure_signed(origin)?;
			Self::execute_register_member(staker, network, public_key, peer_id, bond)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::register_member())]
		pub fn sudo_register_member(
			origin: OriginFor<T>,
			staker: AccountId,
			network: NetworkId,
			public_key: PublicKey,
			peer_id: PeerId,
			bond: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::execute_register_member(staker, network, public_key, peer_id, bond)
		}

		/// `send_heartbeat`: Updates the last heartbeat time for a member.
		/// # Flow
		///	1. Receives `origin` (caller's account).
		///	2. Ensures the `origin` is signed (authenticated) and retrieves the `member` account.
		///	3. Checks if the member is registered ([`MemberNetwork::<T>::get(&member)`]).
		///	4. Updates the [`Heartbeat::<T>`] storage for the member.
		///	5. Emits [`Event::HeartbeatReceived`].
		///	6. Calls `Self::is_member_online` to check if the member is already online.
		///		1. If not online, calls `Self::member_online` to mark them as online.
		///	7. Returns `Ok(())` if successful.
		#[pallet::call_index(2)]
		#[pallet::weight((<T as Config>::WeightInfo::send_heartbeat(), DispatchClass::Operational))]
		pub fn send_heartbeat(origin: OriginFor<T>) -> DispatchResult {
			let member = ensure_signed(origin)?;
			Self::execute_send_heartbeat(member)
		}

		#[pallet::call_index(3)]
		#[pallet::weight((<T as Config>::WeightInfo::send_heartbeat(), DispatchClass::Operational))]
		pub fn sudo_send_heartbeat(origin: OriginFor<T>, member: AccountId) -> DispatchResult {
			ensure_root(origin)?;
			Self::execute_send_heartbeat(member)
		}

		///  - `unregister_member`: Unregisters a member from the network.
		/// # Flow
		///	1. Receives `origin` (caller's account).
		///	2. Ensures the `origin` is signed (authenticated) and retrieves the `member` account.
		///	3. Retrieves the current `network` of the member ([`MemberNetwork::<T>::take(&member)`]).
		///	4. Calls `Self::unregister_member_from_network` to perform the actual unregistration tasks:
		///	5. Unreserves the member's stake ([`pallet_balances::Pallet::<T>::unreserve`]).
		///	6. Removes data from storage ([`MemberPublicKey::<T>`], [`MemberPeerId::<T>`], [`Heartbeat::<T>`], [`MemberOnline::<T>`]).
		///	7. Emits [`Event::UnRegisteredMember`].
		///	8. Calls `Self::member_offline` to mark the member as offline and calculate weight adjustments.
		///	9. Returns `Ok(())` if successful.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::unregister_member())]
		pub fn unregister_member(origin: OriginFor<T>, member: AccountId) -> DispatchResult {
			let staker = ensure_signed(origin)?;
			Self::execute_unregister_member(staker, member)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::unregister_member())]
		pub fn sudo_unregister_member(
			origin: OriginFor<T>,
			staker: AccountId,
			member: AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::execute_unregister_member(staker, member)
		}
	}

	impl<T: Config> Pallet<T> {
		fn execute_register_member(
			staker: AccountId,
			network: NetworkId,
			public_key: PublicKey,
			peer_id: PeerId,
			bond: BalanceOf<T>,
		) -> DispatchResult {
			let member = public_key.clone().into_account();
			ensure!(MemberStake::<T>::get(&member) == 0, Error::<T>::StillStaked);
			ensure!(bond >= T::MinStake::get(), Error::<T>::BondBelowMinStake);
			pallet_balances::Pallet::<T>::reserve(&staker, bond)?;
			MemberStake::<T>::insert(&member, bond);
			MemberStaker::<T>::insert(&member, staker);
			MemberNetwork::<T>::insert(&member, network);
			MemberPublicKey::<T>::insert(&member, public_key);
			MemberPeerId::<T>::insert(&member, peer_id);
			MemberRegistered::<T>::insert(&member, ());
			Self::deposit_event(Event::RegisteredMember(member, network, peer_id));
			Ok(())
		}
		fn execute_send_heartbeat(member: AccountId) -> DispatchResult {
			ensure!(
				MemberStake::<T>::get(&member) >= T::MinStake::get(),
				Error::<T>::BondBelowMinStake
			);
			let network = MemberNetwork::<T>::get(&member).ok_or(Error::<T>::NotMember)?;
			if !Self::is_member_online(&member) {
				Self::member_online(&member, network);
			}
			Heartbeat::<T>::insert(&member, ());
			TimedOut::<T>::mutate(|members| members.retain(|m| *m != member));
			Self::deposit_event(Event::HeartbeatReceived(member));
			Ok(())
		}
		fn execute_unregister_member(staker: AccountId, member: AccountId) -> DispatchResult {
			ensure!(MemberStaker::<T>::get(&member) == Some(staker), Error::<T>::NotStaker);
			let network = MemberNetwork::<T>::get(&member).ok_or(Error::<T>::NotMember)?;
			ensure!(MemberRegistered::<T>::take(&member).is_some(), Error::<T>::NotRegistered);
			Self::unstake_member(&member);
			Self::deposit_event(Event::UnRegisteredMember(member, network));
			Ok(())
		}
		/// Handles periodic heartbeat checks and manages member online/offline statuses.
		pub(crate) fn timeout_heartbeats() -> Weight {
			let timed_out_members = TimedOut::<T>::take();
			let heartbeats = Heartbeat::<T>::drain();
			let mut next_timed_out =
				Vec::with_capacity(timed_out_members.len() + heartbeats.size_hint().0);
			let mut num_timeouts = 0u32;

			for member in timed_out_members.into_iter() {
				if num_timeouts >= T::MaxTimeoutsPerBlock::get() {
					next_timed_out.push(member);
					continue;
				}

				if let Some(network) = MemberNetwork::<T>::get(&member) {
					Self::member_offline(&member, network);
					num_timeouts += 1;
				} else {
					next_timed_out.push(member);
				}
			}

			// Extend with Heartbeat members
			next_timed_out.extend(heartbeats.map(|(m, _)| m));

			// Update storage
			TimedOut::<T>::put(next_timed_out);

			// Return weight consumed
			<T as Config>::WeightInfo::timeout_heartbeats(num_timeouts)
		}
		///  Marks a member as online.
		/// # Flow
		///	1. Receives `member` (account of the member) and `network` (NetworkId).
		///	2. Inserts `member` into [`MemberOnline::<T>`] storage.
		///	3. Emits [`Event::MemberOnline`].
		///	4. Calls `Elections::member_online` to notify the election system of the member's online status.
		fn member_online(member: &AccountId, network: NetworkId) {
			MemberOnline::<T>::insert(member.clone(), ());
			Self::deposit_event(Event::MemberOnline(member.clone()));
			T::Elections::member_online(member, network);
		}

		///  Marks a member as offline.
		/// # Flow
		///	1. Receives `member` (account of the member) and `network` (NetworkId).
		///	2. Removes `member` from [`MemberOnline::<T>`] storage.
		///	3. Emits [`Event::MemberOffline]`.
		fn member_offline(member: &AccountId, network: NetworkId) {
			MemberOnline::<T>::remove(member);
			Self::deposit_event(Event::MemberOffline(member.clone()));
			T::Elections::member_offline(member, network);
		}

		/// Retrieves the heartbeat timeout value.
		///
		/// This function fetches the timeout duration for heartbeats from the associated configuration.
		/// The heartbeat timeout is used to determine the maximum allowed duration between heartbeats before considering the node as inactive.
		pub fn get_heartbeat_timeout() -> BlockNumberFor<T> {
			T::HeartbeatTimeout::get()
		}

		/// Retrieves the minimum stake value.
		///
		/// This function fetches the minimum required stake from the associated configuration.
		/// The minimum stake is the least amount of tokens required to participate in staking.
		pub fn get_min_stake() -> BalanceOf<T> {
			T::MinStake::get()
		}
	}

	impl<T: Config> MembersInterface for Pallet<T> {
		/// Retrieves the stake of a specific member.
		fn member_stake(account: &AccountId) -> BalanceOf<T> {
			MemberStake::<T>::get(account)
		}

		/// Retrieves the peer ID of a specific member.
		fn member_peer_id(account: &AccountId) -> Option<PeerId> {
			MemberPeerId::<T>::get(account)
		}

		/// Retrieves the public key of a specific member.
		fn member_public_key(account: &AccountId) -> Option<PublicKey> {
			MemberPublicKey::<T>::get(account)
		}

		/// Checks if a specific member is online.
		fn is_member_online(account: &AccountId) -> bool {
			MemberOnline::<T>::get(account).is_some()
		}

		fn is_member_registered(account: &AccountId) -> bool {
			MemberRegistered::<T>::get(account).is_some()
		}

		/// Retrieves the total stake of all members.
		fn total_stake() -> u128 {
			let mut total: u128 = 0;
			for stake in MemberStake::<T>::iter() {
				total = total.saturating_add(stake.1);
			}
			total
		}

		/// Transfers a specified amount of stake from one account to another.
		///
		/// This function checks if the `from` account has sufficient stake before proceeding with the transfer.
		/// It unreserves the specified amount from the `from` account, transfers it to the `to` account,
		/// and updates the `from` account's remaining stake.
		/// Returns [`Error::<T>::StakedBelowTransferAmount`] if the `from` account does not have enough stake.
		fn transfer_stake(from: &AccountId, to: &AccountId, amount: Balance) -> DispatchResult {
			let total_stake = MemberStake::<T>::get(from);
			let remaining_stake =
				total_stake.checked_sub(amount).ok_or(Error::<T>::StakedBelowTransferAmount)?;
			pallet_balances::Pallet::<T>::unreserve(from, amount);
			pallet_balances::Pallet::<T>::transfer(
				from,
				to,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			MemberStake::<T>::insert(from, remaining_stake);
			Ok(())
		}

		fn unstake_member(member: &AccountId) {
			if T::Shards::is_shard_member(member) {
				return;
			}
			if Self::is_member_registered(member) {
				return;
			}
			if let Some(staker) = MemberStaker::<T>::take(member) {
				let stake = MemberStake::<T>::take(member);
				pallet_balances::Pallet::<T>::unreserve(&staker, stake);
			}
			MemberNetwork::<T>::remove(member);
			MemberPeerId::<T>::remove(member);
			MemberPublicKey::<T>::remove(member);
		}
	}
}
