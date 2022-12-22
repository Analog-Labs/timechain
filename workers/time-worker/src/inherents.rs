use crate::TW_LOG;
use log::{error, info};
use parking_lot::Mutex;
use sp_inherents::{Error, InherentData, InherentDataProvider, InherentIdentifier};
use std::{collections::HashMap, sync::Arc};
use time_primitives::inherents::{InherentError, TimeTssKey, INHERENT_IDENTIFIER};

lazy_static::lazy_static! {
	static ref TIME_TSS_STORAGE: Arc<Mutex<TimeInherentTssDataProvider>> = {
		Arc::new(Mutex::new(TimeInherentTssDataProvider::default()))
	};
}

pub fn update_shared_group_key(id: u64, key: [u8; 32]) {
    info!(target: TW_LOG, "New group key provided: {:?} for id: {}", key, id);
    TIME_TSS_STORAGE.lock().new_group_key(id, key);
}

/// Our inherent data provider for runtime
#[derive(Debug, Clone, Default)]
pub struct TimeInherentTssDataProvider {
	pub(crate) group_keys: HashMap<u64, [u8; 32]>,
	pub(crate) current_set_id: u64,
}

impl TimeInherentTssDataProvider {
	pub fn new_group_key(&mut self, set_id: u64, new_key: [u8; 32]) {
		self.current_set_id = set_id;
		self.group_keys.insert(set_id, new_key);
	}
}

#[async_trait::async_trait]
impl InherentDataProvider for TimeInherentTssDataProvider {
	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		if let Some(group_key) = self.group_keys.get(&self.current_set_id) {
			inherent_data.put_data(
				INHERENT_IDENTIFIER,
				&TimeTssKey { group_key: *group_key, set_id: self.current_set_id },
			)
		} else {
			inherent_data.put_data(
				INHERENT_IDENTIFIER,
				&TimeTssKey { group_key: [0u8; 32], set_id: self.current_set_id },
			)
		}
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), Error>> {
		// Check if this error belongs to us.
		if *identifier != INHERENT_IDENTIFIER {
			return None
		}

		match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
			InherentError::InvalidGroupKey(wrong_key) =>
				if wrong_key.group_key == [0u8; 32] {
					error!(
						target: TW_LOG,
						"Invalid Group Key: {:?} in Imported Block", wrong_key.group_key
					);
					Some(Err(sp_inherents::Error::Application(Box::from(
						InherentError::InvalidGroupKey(wrong_key),
					))))
				} else {
					error!(target: TW_LOG, "No Group Key found in Imported Block");
					Some(Err(sp_inherents::Error::Application(Box::from(
						InherentError::InvalidGroupKey(wrong_key),
					))))
				},
			InherentError::WrongInherentCall => {
				error!(target: TW_LOG, "Invalid Call inserted in block");
				Some(Err(sp_inherents::Error::Application(Box::from(
					InherentError::WrongInherentCall,
				))))
			},
		}
	}
}
