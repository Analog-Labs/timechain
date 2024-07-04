//! # Analog Timechain Documentation
//!
//! This crate is a *minimal* but *always-accurate* documentation
//! of the Timechain protocol.
//!
//! ## The Timechain Protocol
//!
//! The Analog Timechain is a substrate based solochain. It utilizes
//! Babe and Grandpa to power its `timechain_node` and [`timechain_runtime`].
//!
//! On top of that it runs the Timechain protocol to attest and relay data
//! between various chains. This protocol is executed by shards of [`chronicle`] nodes.
//!
//! ## Custom pallets
//!
//! - [`pallet_elections`]
//! - [`pallet_members`]
//! - [`pallet_networks`]
//! - [`pallet_shards`]
//! - [`pallet_tasks`]
//! - [`pallet_timegraph`]
//!
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![doc(html_favicon_url = "https://www.analog.one/images/favicon.ico")]
#![doc(html_logo_url = "https://www.analog.one/images/logo.svg")]
#![doc(issue_tracker_base_url = "https://github.com/Analog-Labs/timechain/issues")]
