pub mod local_state_struct;
pub mod signverify;
pub mod submit_to_timechain;
pub mod tss_event_handler;
pub mod tss_event_handler_helper;
pub mod tss_event_model;
pub mod tss_service;
pub mod utils;

pub use frost_dalek;
pub use rand;

pub const DEFUALT_TSS_TOTAL_NODES: u32 = 3;
pub const DEFUALT_TSS_THRESHOLD: u32 = 2;
