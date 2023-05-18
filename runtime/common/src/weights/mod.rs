//! Expose the auto generated weight files.

// TODO: replace with target hardware runs
pub mod block_weights;
pub mod extrinsic_weights;

pub use block_weights::BlockExecutionWeight;
pub use extrinsic_weights::ExtrinsicBaseWeight;
