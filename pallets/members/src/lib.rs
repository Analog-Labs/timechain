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
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{IdentifyAccount, Zero};
	use sp_std::collections::btree_map::BTreeMap;
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
		/// Ensured origin for calls to register/unregister members
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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

		/// [`Event::MembersOffline`] members offline statuses changes
		MembersOffline(Vec<AccountId>),

		/// [`Event::UnRegisteredMember`] member unregistration event.
		UnRegisteredMember(AccountId, NetworkId),
	}

	///  Define possible errors that can occur during pallet operations.
	#[pallet::error]
	pub enum Error<T> {
		/// Not a member.
		NotMember,
		/// Member not registered.
		NotRegistered,
		/// Heartbeat already submitted for timeout period
		AlreadySubmittedHeartbeat,
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
		///	2. Ensures the `origin` is AdminOrigin (authenticated).
		///	3. Validates the `public_key` against the `origin` account.
		///	4. Checks if the member is already registered and unregisters them if necessary.
		///	5. Inserts member data into respective storage maps ([`MemberNetwork::<T>`], [`MemberPublicKey::<T>`], [`MemberPeerId::<T>`], [`MemberStake::<T>`], [`Heartbeat::<T>`]).
		///	6. Marks the member as online ([`MemberOnline::<T>`]).
		///	7. Emits [`Event::RegisteredMember`].
		///	8. Calls `Self::member_online` to notify the network election system.
		///	9. Returns `Ok(())` if successful.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_member())]
		pub fn register_member(
			origin: OriginFor<T>,
			network: NetworkId,
			public_key: PublicKey,
			peer_id: PeerId,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::execute_register_member(network, public_key, peer_id)
		}

		///  - `unregister_member`: Unregisters a member from the network.
		/// # Flow
		///	1. Receives `origin` (caller's account).
		///	2. Ensures the `origin` is signed (authenticated) and retrieves the `member` account.
		///	3. Retrieves the current `network` of the member ([`MemberNetwork::<T>::take(&member)`]).
		///	4. Calls `Self::unregister_member_from_network` to perform the actual unregistration tasks:
		///	5. Removes data from storage ([`MemberPublicKey::<T>`], [`MemberPeerId::<T>`], [`Heartbeat::<T>`], [`MemberOnline::<T>`]).
		///	6. Emits [`Event::UnRegisteredMember`].
		///	7. Calls `Self::member_offline` to mark the member as offline and calculate weight adjustments.
		///	8. Returns `Ok(())` if successful.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::unregister_member())]
		pub fn unregister_member(origin: OriginFor<T>, member: AccountId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::execute_unregister_member(member)
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
		#[pallet::weight((
			<T as Config>::WeightInfo::send_heartbeat(),
			DispatchClass::Operational,
			Pays::No
		))]
		pub fn send_heartbeat(origin: OriginFor<T>) -> DispatchResult {
			let member = ensure_signed(origin)?;
			Self::execute_send_heartbeat(member)
		}
	}

	impl<T: Config> Pallet<T> {
		fn execute_register_member(
			network: NetworkId,
			public_key: PublicKey,
			peer_id: PeerId,
		) -> DispatchResult {
			let member = public_key.clone().into_account();
			MemberNetwork::<T>::insert(&member, network);
			MemberPublicKey::<T>::insert(&member, public_key);
			MemberPeerId::<T>::insert(&member, peer_id);
			MemberRegistered::<T>::insert(&member, ());
			Self::deposit_event(Event::RegisteredMember(member, network, peer_id));
			Ok(())
		}
		fn execute_send_heartbeat(member: AccountId) -> DispatchResult {
			ensure!(Heartbeat::<T>::get(&member).is_none(), Error::<T>::AlreadySubmittedHeartbeat);
			let network = MemberNetwork::<T>::get(&member).ok_or(Error::<T>::NotMember)?;
			if !Self::is_member_online(&member) {
				Self::member_online(&member, network);
			}
			Heartbeat::<T>::insert(&member, ());
			TimedOut::<T>::mutate(|members| members.retain(|m| *m != member));
			Self::deposit_event(Event::HeartbeatReceived(member));
			Ok(())
		}
		fn execute_unregister_member(member: AccountId) -> DispatchResult {
			let network = MemberNetwork::<T>::get(&member).ok_or(Error::<T>::NotMember)?;
			ensure!(MemberRegistered::<T>::take(&member).is_some(), Error::<T>::NotRegistered);
			Self::do_unregister_member(&member);
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

			let mut current_timed_out = BTreeMap::<NetworkId, Vec<AccountId>>::new();
			for member in timed_out_members.into_iter() {
				if num_timeouts >= T::MaxTimeoutsPerBlock::get() {
					next_timed_out.push(member);
					continue;
				}

				if let Some(network) = MemberNetwork::<T>::get(&member) {
					current_timed_out.entry(network).or_default().push(member);
					num_timeouts += 1;
				} else {
					next_timed_out.push(member);
				}
			}
			for (network, members) in current_timed_out {
				Self::members_offline(members, network);
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

		///  Marks members as offline.
		/// # Flow
		///	1. Receives `members` (accounts of the members) and `network` (NetworkId).
		///	2. Removes `members` from [`MemberOnline::<T>`] storage.
		///	3. Emits [`Event::MembersOffline]`.
		fn members_offline(members: Vec<AccountId>, network: NetworkId) {
			for m in &members {
				MemberOnline::<T>::remove(m);
			}
			Self::deposit_event(Event::MembersOffline(members.clone()));
			T::Elections::members_offline(members, network);
		}

		/// Retrieves the heartbeat timeout value.
		///
		/// This function fetches the timeout duration for heartbeats from the associated configuration.
		/// The heartbeat timeout is used to determine the maximum allowed duration between heartbeats before considering the node as inactive.
		pub fn get_heartbeat_timeout() -> BlockNumberFor<T> {
			T::HeartbeatTimeout::get()
		}
	}

	impl<T: Config> MembersInterface for Pallet<T> {
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

		fn do_unregister_member(account: &AccountId) {
			if !T::Shards::is_shard_member(account) {
				MemberNetwork::<T>::remove(account);
				MemberPeerId::<T>::remove(account);
				MemberPublicKey::<T>::remove(account);
			}
		}
	}
}
