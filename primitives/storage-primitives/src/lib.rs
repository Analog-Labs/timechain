#![cfg_attr(not(feature = "std"), no_std)]
pub type Frequency = u64;
pub type TaskId = u64;

sp_api::decl_runtime_apis! {}
