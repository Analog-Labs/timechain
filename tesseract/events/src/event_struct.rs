use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum EventType {
	TSSEvent,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Event {
	pub event_type: EventType,
	pub data: Vec<u8>,
}
