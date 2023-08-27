use crate::mock::*;

// register member emits event

#[test]
fn register_member_inserts_into_unassigned() {
	assert!(true);
}
// register inserts into MemberNetwork
// register cannot be called if already member

// assign_member resets heartbeat
// assign_member removes from unassigned

// send_heartbeat emits event
// send_heartbeat resets heartbeat
// send_heartbeat fails if not member
// send_heartbeat fails if not assigned

// assignment + delay without send_heartbeat leads to member offline

// new_shard_members(x, _) only returns Some(y) if y.len() = x
