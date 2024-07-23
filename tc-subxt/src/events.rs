use scale_decode::DecodeAsType;
use subxt::ext::subxt_core::events::StaticEvent;
use time_primitives::TaskResult as PrimitiveTaskResult;

#[derive(DecodeAsType, Debug)]
pub struct TaskResult(pub u64, pub PrimitiveTaskResult);
impl StaticEvent for TaskResult {
	const PALLET: &'static str = "Tasks";
	const EVENT: &'static str = "TaskResult";
}
#[derive(DecodeAsType, Debug)]
pub struct TaskCreated(pub u64);
impl StaticEvent for TaskCreated {
	const PALLET: &'static str = "Tasks";
	const EVENT: &'static str = "TaskCreated";
}

#[derive(DecodeAsType, Debug)]
pub struct GatewayRegistered(pub u16, pub [u8; 20], pub u64);
impl StaticEvent for GatewayRegistered {
	const PALLET: &'static str = "Tasks";
	const EVENT: &'static str = "GatewayRegistered";
}

#[derive(DecodeAsType, Debug)]
pub struct UnRegisteredMember(pub [u8; 32], pub u64);
impl StaticEvent for UnRegisteredMember {
	const PALLET: &'static str = "Members";
	const EVENT: &'static str = "UnRegisteredMember";
}
