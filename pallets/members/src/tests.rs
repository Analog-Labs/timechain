use crate::mock::*;
use crate::{Error, Event, Heartbeat, MemberNetwork, MemberPeerId};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::{AccountId, MemberStorage, Network};

const A: [u8; 32] = [1u8; 32];

#[test]
fn register_member_works() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			Network::Ethereum,
			A,
		));
		System::assert_last_event(
			Event::<Test>::RegisteredMember(a.clone(), Network::Ethereum, A).into(),
		);
		assert_eq!(Members::member_peer_id(a.clone()), Some(A));
		assert_eq!(MemberPeerId::<Test>::get(&a), Some(A));
		assert_eq!(MemberNetwork::<Test>::get(&a), Some(Network::Ethereum));
		assert!(Heartbeat::<Test>::get(&a).unwrap().is_online);
		assert_eq!(Heartbeat::<Test>::get(&a).unwrap().block, 1);
	});
}

#[test]
fn cannot_register_member_twice() {
	new_test_ext().execute_with(|| {
		assert_ok!(Members::register_member(
			RawOrigin::Signed(A.into()).into(),
			Network::Ethereum,
			A,
		));
		assert_noop!(
			Members::register_member(RawOrigin::Signed(A.into()).into(), Network::Ethereum, A,),
			Error::<Test>::AlreadyMember
		);
	});
}

#[test]
fn send_heartbeat_works() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			Network::Ethereum,
			A,
		));
		roll_to(5);
		assert_ok!(Members::send_heartbeat(RawOrigin::Signed(a.clone()).into()));
		System::assert_last_event(Event::<Test>::HeartbeatReceived(a.clone()).into());
		assert!(Heartbeat::<Test>::get(&a).unwrap().is_online);
		assert_eq!(Heartbeat::<Test>::get(&a).unwrap().block, 5);
	});
}

#[test]
fn no_heartbeat_sets_member_offline_after_timeout() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			Network::Ethereum,
			A,
		));
		roll_to(11);
		assert!(!Heartbeat::<Test>::get(&a).unwrap().is_online);
		assert_eq!(Heartbeat::<Test>::get(&a).unwrap().block, 1);
	});
}

#[test]
fn send_heartbeat_sets_member_back_online_after_timeout() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			Network::Ethereum,
			A,
		));
		roll_to(11);
		assert!(!Heartbeat::<Test>::get(&a).unwrap().is_online);
		assert_eq!(Heartbeat::<Test>::get(&a).unwrap().block, 1);
		assert_ok!(Members::send_heartbeat(RawOrigin::Signed(a.clone()).into()));
		assert!(Heartbeat::<Test>::get(&a).unwrap().is_online);
		assert_eq!(Heartbeat::<Test>::get(&a).unwrap().block, 11);
	});
}

#[test]
fn cannot_send_heartbeat_if_not_member() {
	new_test_ext().execute_with(|| {
		let a: AccountId = A.into();
		assert_noop!(
			Members::send_heartbeat(RawOrigin::Signed(a.clone()).into()),
			Error::<Test>::NotMember
		);
	});
}
