use std::collections::HashMap;

use borsh::BorshSerialize;
use events::event_struct::{Event, EventType};
use frost_dalek::{
	keygen::{Coefficients, RoundOne, SecretShare},
	DistributedKeyGeneration, Parameters, Participant,
};

use crate::tss_event_model::{
	PublishPeerIDCall, ReceiveParamsWithPeerCall, ResetTSSCall, TSSData, TSSEventType,
};

use std::time::{SystemTime, UNIX_EPOCH};

pub fn make_gossip_tss_data(
	peer_id: String,
	internal_data: Vec<u8>,
	tss_type: TSSEventType,
) -> Result<Vec<u8>, String> {
	let tss_event = TSSData {
		peer_id,
		tss_data: internal_data,
		tss_event_type: tss_type,
	};

	tss_event
		.try_to_vec()
		.map_err(|e| format!("Unable to convert data to vector: {e}"))
}

pub async fn make_hashmap_for_secret_share(
	secret_shares: &Vec<SecretShare>,
) -> HashMap<u32, SecretShare> {
	secret_shares
		.iter()
		.map(|x| (x.index, x.clone()))
		.collect::<HashMap<u32, SecretShare>>()
}

pub fn make_participant(params: Parameters, index: u32) -> (Participant, Coefficients) {
	Participant::new(&params, index)
}

pub fn get_participant_index(peer_id: String, other_peer_id: &Vec<String>) -> u32 {
	let mut other_peer_list = other_peer_id.clone();
	other_peer_list.sort();

	let list_length = other_peer_list.len();
	for index in 0..list_length {
		if &peer_id <= &other_peer_list[index] {
			return (index + 1) as u32;
		}
	}
	return (list_length + 1) as u32;
}

pub fn get_publish_peer_id_msg(local_peer: String) -> Result<Vec<u8>, String> {
	let start = SystemTime::now();
	if let Ok(since_the_epoch) = start.duration_since(UNIX_EPOCH) {
		let data = PublishPeerIDCall {
			peer_id: local_peer,
			random: since_the_epoch.as_millis().to_string(),
		};

		data.try_to_vec().map_err(|e| format!("Unable to convert data to vector: {e}"))
	} else {
		Err("Unable to get time difference".to_string())
	}
}

pub fn get_reset_tss_msg(reason: String) -> Result<Vec<u8>, String> {
	let start = SystemTime::now();
	if let Ok(since_the_epoch) = start.duration_since(UNIX_EPOCH) {
		let data = ResetTSSCall {
			reason,
			random: since_the_epoch.as_millis().to_string(),
		};

		data.try_to_vec().map_err(|e| format!("Unable to convert data to vector: {e}"))
	} else {
		Err("Unable to get time difference".to_string())
	}
}

pub fn get_receive_params_msg(local_peer: String, params: Parameters) -> Result<Vec<u8>, String> {
	let start = SystemTime::now();
	if let Ok(since_the_epoch) = start.duration_since(UNIX_EPOCH) {
		let data = ReceiveParamsWithPeerCall {
			peer_id: local_peer,
			random: since_the_epoch.as_millis().to_string(),
			params,
		};

		data.try_to_vec().map_err(|e| format!("Unable to convert data to vector: {e}"))
	} else {
		Err("Unable to get time difference".to_string())
	}
}

pub fn round_one_state(
	params: &Parameters,
	index: &u32,
	coefficients: &Coefficients,
	participants: &mut Vec<Participant>,
) -> Result<DistributedKeyGeneration<RoundOne>, Box<Vec<u32>>> {
	match DistributedKeyGeneration::<_>::new(params, index, coefficients, participants) {
		Ok(dkg) => Ok(dkg),
		Err(e) => Err(Box::new(e)),
	}
}
