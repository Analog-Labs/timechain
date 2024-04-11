//! Mock results for lengths from 31 to MAX_PAYLOAD_LEN
//! Generated via `tests::bench_result_helper`
use time_primitives::{Payload, TaskResult};

#[derive(Clone)]
pub struct FullSig {
	payload: time_primitives::Payload,
	signature: [u8; 64],
}

pub const MIN_PAYLOAD_LENGTH: u32 = 31;
fn payloads() -> Vec<FullSig> {
	vec![FullSig {
		payload: Payload::Hashed([
			11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91,
			0, 219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
		]),
		signature: [
			6, 7, 9, 187, 47, 68, 0, 246, 107, 215, 169, 76, 121, 8, 85, 213, 42, 253, 100, 32, 62,
			87, 85, 101, 146, 126, 200, 74, 76, 101, 188, 229, 30, 17, 202, 255, 105, 25, 145, 174,
			219, 202, 54, 185, 97, 39, 171, 219, 81, 123, 73, 35, 124, 32, 124, 148, 155, 133, 40,
			73, 165, 196, 167, 130,
		],
	}]
}
pub fn mock_result(length: u32) -> ([u8; 33], TaskResult) {
	let index: usize = (length - MIN_PAYLOAD_LENGTH).try_into().unwrap();
	let FullSig { payload, signature } = &payloads().clone()[index];
	(
		[
			2, 36, 79, 43, 160, 29, 26, 4, 168, 242, 35, 104, 66, 1, 179, 183, 189, 197, 92, 84, 2,
			101, 52, 245, 230, 250, 199, 131, 188, 204, 228, 70, 248,
		],
		TaskResult {
			shard_id: 0,
			payload: payload.clone(),
			signature: *signature,
		},
	)
}
