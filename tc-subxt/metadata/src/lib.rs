#[cfg(not(feature = "testnet"))]
compile_error!("GMP is currently not supported on mainnet.");

include!(concat!(std::env!("OUT_DIR"), "/metadata.rs"));
