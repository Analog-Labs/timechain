use anyhow::Result;

pub(crate) mod admin;
pub(crate) mod sol;
pub(crate) mod ufloat;

pub use crate::admin::AdminConnector;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Address([u8; 20]);

impl From<[u8; 32]> for Address {
	fn from(addr: [u8; 32]) -> Self {
		Self(addr[12..].try_into().unwrap())
	}
}

impl From<Address> for [u8; 32] {
	fn from(addr: Address) -> Self {
		let mut r = [0; 32];
		r[12..].copy_from_slice(&addr.0);
		r
	}
}

impl From<[u8; 20]> for Address {
	fn from(addr: [u8; 20]) -> Self {
		Self(addr)
	}
}

impl From<Address> for [u8; 20] {
	fn from(addr: Address) -> Self {
		addr.0
	}
}

impl From<alloy_primitives::Address> for Address {
	fn from(addr: alloy_primitives::Address) -> Self {
		Self(addr.into())
	}
}

impl From<Address> for alloy_primitives::Address {
	fn from(addr: Address) -> Self {
		Self::from(addr.0)
	}
}

impl std::fmt::Display for Address {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		alloy_primitives::Address::from(*self).fmt(f)
	}
}

impl std::str::FromStr for Address {
	type Err = anyhow::Error;

	fn from_str(addr: &str) -> Result<Self> {
		Ok(alloy_primitives::Address::from_str(addr)?.into())
	}
}
