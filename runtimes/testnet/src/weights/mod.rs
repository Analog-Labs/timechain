//! A list of the different weight modules for our runtime.

pub mod dmail;
pub mod elections;
pub mod members;
pub mod networks;
pub mod shards;
pub mod tasks;
pub mod timegraph;

// FRAME
// do not uncomment until configured in runtime
//pub mod babe;
//pub mod grandpa;
pub mod balances;
pub mod system;
pub mod timestamp;
