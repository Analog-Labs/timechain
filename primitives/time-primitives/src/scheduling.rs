// TODO: move relevant types/traits from abstraction.rs to here

pub trait GetNetworkTimeout<Network, BlockNumber> {
	fn get_network_timeout(network: Network) -> BlockNumber;
}

impl<Network, BlockNumber: Default> GetNetworkTimeout<Network, BlockNumber> for () {
	fn get_network_timeout(_network: Network) -> BlockNumber {
		Default::default()
	}
}
