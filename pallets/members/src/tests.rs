use crate::mock::*;
use crate::{Error, Event, Heartbeat, MemberNetwork, MemberOnline, MemberPeerId, MemberStake};

use polkadot_sdk::{frame_support, frame_system, sp_runtime};

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::{DispatchError, DispatchResult, ModuleError};

use time_primitives::{AccountId, MembersInterface, NetworkId};

const A: [u8; 32] = [1u8; 32];
const C: [u8; 32] = [3u8; 32];
const ETHEREUM: NetworkId = 0;

fn register_member(staker: AccountId, pubkey: [u8; 32], stake: u128) -> DispatchResult {
	Members::register_member(
		RawOrigin::Signed(staker).into(),
		ETHEREUM,
		pubkey_from_bytes(pubkey),
		pubkey,
		stake,
	)
}

fn unregister_member(staker: AccountId, pubkey: [u8; 32]) -> DispatchResult {
	Members::unregister_member(RawOrigin::Signed(staker).into(), pubkey.into())
}

fn send_heartbeat(pubkey: [u8; 32]) -> DispatchResult {
	Members::send_heartbeat(RawOrigin::Signed(pubkey.into()).into())
}

#[test]
fn register_member_works() {
	let a: AccountId = A.into();
	new_test_ext().execute_with(|| {
		assert_ok!(register_member(a.clone(), A, 5));
		System::assert_last_event(Event::<Test>::RegisteredMember(a.clone(), ETHEREUM, A).into());
		assert_eq!(Members::member_peer_id(&a), Some(A));
		assert_eq!(MemberStake::<Test>::get(&a), 5);
		assert_eq!(Balances::reserved_balance(&a), 5);
		assert_eq!(Balances::free_balance(&a), 9999999995);
		assert_eq!(MemberPeerId::<Test>::get(&a), Some(A));
		assert_eq!(MemberNetwork::<Test>::get(&a), Some(ETHEREUM));
	});
}

#[test]
fn cannot_register_member_without_balance() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			register_member(C.into(), C, 5),
			DispatchError::Module(ModuleError {
				index: 1,
				error: [2, 0, 0, 0],
				message: Some("InsufficientBalance")
			})
		);
	});
}

#[test]
fn cannot_register_member_below_min_stake_bond() {
	new_test_ext().execute_with(|| {
		assert_noop!(register_member(A.into(), A, 4), Error::<Test>::BondBelowMinStake);
	});
}

#[test]
fn register_member_replaces_previous_call() {
	let a: AccountId = A.into();
	new_test_ext().execute_with(|| {
		assert_ok!(register_member(a.clone(), A, 5));
		assert_eq!(Members::member_peer_id(&a), Some(A));
		assert_eq!(MemberStake::<Test>::get(&a), 5);
		assert_eq!(Balances::reserved_balance(&a), 5);
		assert_eq!(Balances::free_balance(&a), 9999999995);
		assert_eq!(MemberPeerId::<Test>::get(&a), Some(A));
		assert_eq!(MemberNetwork::<Test>::get(&a), Some(ETHEREUM));
		assert_noop!(register_member(a.clone(), A, 10), Error::<Test>::StillStaked);
	});
}

#[test]
fn send_heartbeat_works() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(register_member(a.clone(), A, 5));
		roll_to(5);
		assert_ok!(send_heartbeat(A));
		assert_ok!(send_heartbeat(A));
		System::assert_last_event(Event::<Test>::HeartbeatReceived(a.clone()).into());
		assert!(MemberOnline::<Test>::get(&a).is_some());
		assert!(Heartbeat::<Test>::get(&a).is_some());
	});
}

#[test]
fn no_heartbeat_sets_member_offline_after_timeout() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(register_member(a.clone(), A, 5));
		assert_ok!(send_heartbeat(A));
		roll_to(10);
		assert!(MemberOnline::<Test>::get(&a).is_some());
		roll_to(20);
		assert!(MemberOnline::<Test>::get(&a).is_none());
	});
}

#[test]
fn send_heartbeat_sets_member_back_online_after_timeout() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(register_member(a.clone(), A, 5));
		roll_to(20);
		assert!(MemberOnline::<Test>::get(&a).is_none());
		assert_ok!(send_heartbeat(A));
		assert!(MemberOnline::<Test>::get(&a).is_some());
		assert!(Heartbeat::<Test>::get(&a).is_some());
	});
}

#[test]
fn cannot_send_heartbeat_if_not_registered() {
	new_test_ext().execute_with(|| {
		assert_noop!(send_heartbeat(A), Error::<Test>::NotRegistered);
	});
}

#[test]
fn cannot_unregister_if_not_registered() {
	new_test_ext().execute_with(|| {
		assert_noop!(unregister_member(A.into(), A), Error::<Test>::NotRegistered);
	});
}

#[test]
fn cannot_unregister_twice() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(register_member(a.clone(), A, 5));
		assert_ok!(unregister_member(a.clone(), A));
		assert_noop!(unregister_member(a, A), Error::<Test>::NotRegistered);
	});
}

#[test]
fn unregister_member_works() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(register_member(a.clone(), A, 5));
		assert_ok!(unregister_member(a.clone(), A));
		System::assert_last_event(Event::<Test>::UnRegisteredMember(a.clone(), ETHEREUM).into());
		assert_eq!(Members::member_peer_id(&a), None);
		assert_eq!(MemberStake::<Test>::get(&a), 0);
		assert_eq!(Balances::reserved_balance(&a), 0);
		assert_eq!(Balances::free_balance(&a), 10000000000);
		assert_eq!(MemberPeerId::<Test>::get(&a), None);
		assert_eq!(MemberNetwork::<Test>::get(&a), None);
		assert!(Heartbeat::<Test>::get(&a).is_none());
	});
}
