pub(crate) mod validator;

pub mod time_protocol_name {
	use sc_network::{config::NonDefaultSetConfig, types::ProtocolName};

	/// time gossip protocol name suffix.
	const GOSSIP_NAME: &str = "/time/1";

	/// Name of the time gossip protocol used by time-workers.
	///
	/// Must be registered towards the networking in order for time gossip to properly function.
	pub fn gossip_protocol_name() -> ProtocolName {
		ProtocolName::Static(GOSSIP_NAME)
	}
	/// Returns the configuration value to put in
	/// [`sc_network::config::NetworkConfiguration::extra_sets`].
	/// For standard protocol name see [`gossip_protocol_name::standard_name`].
	pub fn time_peers_set_config(protocol_name: ProtocolName) -> NonDefaultSetConfig {
		let mut cfg = NonDefaultSetConfig::new(protocol_name, 1024 * 1024);

		cfg.allow_non_reserved(25, 25);
		cfg
	}
}
