use convert_case::{Case, Casing};
use hex_literal::hex;
use serde::{Deserialize, Serialize};

use polkadot_sdk::*;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::{config::TelemetryEndpoints, ChainType};

use sp_authority_discovery::AuthorityId as DiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::UncheckedInto;
use sp_keyring::{AccountKeyring, Ed25519Keyring};
use sp_runtime::Perbill;

use timechain_runtime::WASM_BINARY;

use time_primitives::{AccountId, Balance, Block, ANLOG, SS58_PREFIX, TOKEN_DECIMALS};
use timechain_runtime::{StakerStatus, DAYS};

// MAINNET

// Small endowment to allow admins to work
const MAINNET_PER_ADMIN: Balance = ANLOG * 1000;
// Smaller endowment to allow nodes to rotate keys
const MAINNET_PER_STASH: Balance = ANLOG * 10;

// TESTNET

/// Stash and float for validators
const PER_VALIDATOR_STASH: Balance = ANLOG * 500_000;
const PER_VALIDATOR_UNLOCKED: Balance = ANLOG * 20_000;

/// Stash for community nominations
const PER_NOMINATION: Balance = ANLOG * 180_000;
const PER_NOMINATOR_STASH: Balance = 8 * PER_NOMINATION;

/// Stash and float for chronicles
const PER_CHRONICLE_STASH: Balance = ANLOG * 100_000;

/// Token supply for prefunded admin accounts
const CONTROLLER_SUPPLY: Balance = ANLOG * 50_000;
const PER_COUNCIL_STASH: Balance = ANLOG * 50_000;

/// Minimum needed validators, currently lowered for testing environments
const MIN_VALIDATOR_COUNT: u32 = 1;

/// Default telemetry server for all networks
const DEFAULT_TELEMETRY_URL: &str = "wss://telemetry.analog.one/submit";
const DEFAULT_TELEMETRY_LEVEL: u8 = 1;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Helper to parse genesis keys json
#[derive(serde::Deserialize)]
pub struct GenesisKeysConfig {
	/// Genesis members of on-chain technical committee
	admins: Vec<AccountId>,
	/// Keys used to bootstrap validator session keys.
	/// Will match and register session keys to stashes and self-stake them.
	/// Balance to be staked is controlled by PER_VALIDATOR_UNLOCKED
	bootstraps: Vec<(BabeId, GrandpaId, ImOnlineId, DiscoveryId)>,
	/// Stashes to be used for chronicles, balances controlled by PER_CHRONICLE_STASH
	chronicles: Vec<AccountId>,
	/// Optional controller account that will control all nominates stakes
	controller: Option<AccountId>,
	/// Additional endowed accounts and their balance in ANLOG.
	endowments: Vec<(AccountId, Balance)>,
	/// Stashes intended for community nominations.
	/// Sizing controlled by PER_NOMINATION_STASH
	nominators: Vec<AccountId>,
	/// Stashes intended to be used to run validators.
	/// There has to be at least one stash for every
	/// session key set. Balance controlled by PER_VALIDATOR_STASH
	stakes: Vec<AccountId>,
}

impl Default for GenesisKeysConfig {
	/// Default configuration using know development keys
	fn default() -> Self {
		use AccountKeyring::*;

		GenesisKeysConfig {
			admins: vec![
				Alice.into(),
				Bob.into(),
				Charlie.into(),
				Dave.into(),
				Eve.into(),
				Ferdie.into(),
			],
			bootstraps: vec![(
				Alice.to_raw_public().unchecked_into(),
				Ed25519Keyring::Alice.to_raw_public().unchecked_into(),
				Alice.to_raw_public().unchecked_into(),
				Alice.to_raw_public().unchecked_into(),
			)],
			chronicles: vec![],
			// TODO: Would be better to assign individual controllers
			controller: None,
			endowments: vec![(
				hex!["6d6f646c70792f74727372790000000000000000000000000000000000000000"].into(),
				1_000_000 * ANLOG,
			)],
			nominators: vec![],
			stakes: vec![
				AliceStash.into(),
				BobStash.into(),
				CharlieStash.into(),
				DaveStash.into(),
				EveStash.into(),
				FerdieStash.into(),
			],
		}
	}
}

impl GenesisKeysConfig {
	/// Deserialize genesis key config from json bytes
	pub fn from_json_bytes(json: &[u8]) -> Result<Self, String> {
		serde_json::from_slice(json).map_err(|e| e.to_string())
	}

	/// Generate chain candidate for live deployment
	pub fn to_live(&self) -> Result<ChainSpec, String> {
		if cfg!(feature = "testnet") {
			self.to_chain_spec("analog-testnet", "TANLOG", ChainType::Live, 3, 2)
		} else {
			self.to_chain_spec("analog-timechain", "ANLOG", ChainType::Live, 3, 2)
		}
	}

	/// Generate development chain for supplied sub-identifier
	pub fn to_development(&self, subid: &str) -> Result<ChainSpec, String> {
		let id = "analog-".to_owned() + subid;
		self.to_chain_spec(id.as_str(), "DANLOG", ChainType::Development, 6, 4)
	}

	/// Generate a local chain spec
	pub fn to_local(&self) -> Result<ChainSpec, String> {
		self.to_chain_spec("analog-local", "LANLOG", ChainType::Local, 3, 2)
	}

	/// Generate a chain spec from key config
	#[cfg(not(feature = "testnet"))]
	fn to_chain_spec(
		&self,
		id: &str,
		token_symbol: &str,
		chain_type: ChainType,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<ChainSpec, String> {
		// Determine name from identifier
		let name = id.to_case(Case::Title);

		// Ensure wasm binary is available
		let wasm_binary = WASM_BINARY.expect(
			"Development wasm binary is not available. This means the client is built with \
			 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
			 the flag disabled.",
		);

		// Setup base currency unit name and decimal places
		let mut properties = sc_chain_spec::Properties::new();
		properties.insert("tokenSymbol".into(), token_symbol.into());
		properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
		properties.insert("ss58Format".into(), SS58_PREFIX.into());

		// Add default telemetry for all deployed networks
		let telemetry = if chain_type != ChainType::Local {
			Some(
				TelemetryEndpoints::new(vec![(
					DEFAULT_TELEMETRY_URL.to_string(),
					DEFAULT_TELEMETRY_LEVEL,
				)])
				.expect("Default telemetry url is valid"),
			)
		} else {
			None
		};

		// Convert endowments in config according to token decimals
		let mut endowments = self
			.endowments
			.iter()
			.map(|(addr, bal)| (addr.clone(), bal * ANLOG))
			.collect::<Vec<_>>();

		// Endow council and stashes
		endowments.append(
			&mut self.admins.iter().map(|x| (x.clone(), MAINNET_PER_ADMIN)).collect::<Vec<_>>(),
		);
		endowments.append(
			&mut self.stakes.iter().map(|x| (x.clone(), MAINNET_PER_STASH)).collect::<Vec<_>>(),
		);

		// Load session keys to bootstrap validators from file
		let authorities: Vec<_> = self
			.bootstraps
			.iter()
			.enumerate()
			.map(|(i, x)| {
				(
					self.controller.clone().unwrap_or(self.stakes[i].clone()),
					self.stakes[i].clone(),
					timechain_runtime::SessionKeys {
						babe: x.0.clone(),
						grandpa: x.1.clone(),
						im_online: x.2.clone(),
						authority_discovery: x.3.clone(),
					},
				)
			})
			.collect();

		let genesis_patch = serde_json::json!({
			"balances": {
				"balances": endowments,
			},
			"babe": {
				"epochConfig": timechain_runtime::BABE_GENESIS_EPOCH_CONFIG,
			},
			"session": {
				"keys": authorities,
			},
			"technicalCommittee": {
				"members": Some(self.admins.clone()),
			},
			//"safeMode": {
			//	"enteredUntil":  14 * DAYS,
			//}
		});

		// Put it all together ...
		let mut builder = ChainSpec::builder(wasm_binary, Default::default())
			.with_name(&name)
			.with_id(id)
			.with_protocol_id(id)
			.with_chain_type(chain_type)
			.with_properties(properties)
			.with_genesis_config_patch(genesis_patch)
	        .with_boot_nodes(vec![
				"/dns/bootnode-1.timechain.analog.one/tcp/30333/ws/p2p/12D3KooWB1mphok6BmTmTYZkujiL96LMbHyLCXfS2rTYv7N57cQx".parse().unwrap(),
				"/dns/bootnode-2.timechain.analog.one/tcp/30333/ws/p2p/12D3KooWFSJporNXWo2gooT4x66tKvWVL7T2wcyzGndVvGyNB59X".parse().unwrap()
			]);

		// ... and add optional telemetry
		if let Some(endpoints) = telemetry {
			builder = builder.with_telemetry_endpoints(endpoints);
		}

		// ... to generate chain spec
		Ok(builder.build())
	}

	/// Generate a chain spec from key config
	#[cfg(feature = "testnet")]
	fn to_chain_spec(
		&self,
		id: &str,
		token_symbol: &str,
		chain_type: ChainType,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<ChainSpec, String> {
		// Determine name from identifier
		let name = id.to_case(Case::Title);

		// Ensure wasm binary is available
		let wasm_binary = WASM_BINARY.expect(
			"Development wasm binary is not available. This means the client is built with \
			 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
			 the flag disabled.",
		);

		// Setup base currency unit name and decimal places
		let mut properties = sc_chain_spec::Properties::new();
		properties.insert("tokenSymbol".into(), token_symbol.into());
		properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
		properties.insert("ss58Format".into(), SS58_PREFIX.into());

		// Add default telemetry for all deployed networks
		let telemetry = if chain_type != ChainType::Local {
			Some(
				TelemetryEndpoints::new(vec![(
					DEFAULT_TELEMETRY_URL.to_string(),
					DEFAULT_TELEMETRY_LEVEL,
				)])
				.expect("Default telemetry url is valid"),
			)
		} else {
			None
		};

		// Convert endowments in config according to token decimals
		let mut endowments = self
			.endowments
			.iter()
			.map(|(addr, bal)| (addr.clone(), bal * ANLOG))
			.collect::<Vec<_>>();

		// Budget and endow chronicle stashes
		endowments.append(
			&mut self
				.chronicles
				.iter()
				.map(|x| (x.clone(), PER_CHRONICLE_STASH))
				.collect::<Vec<_>>(),
		);

		// Endow controller if necessary
		if let Some(controller) = self.controller.as_ref() {
			controller_supply = CONTROLLER_SUPPLY;
			endowments.append(&mut vec![(controller.clone(), CONTROLLER_SUPPLY)]);
		}

		// Budget and endow council stashes
		endowments.append(
			&mut self.admins.iter().map(|x| (x.clone(), PER_COUNCIL_STASH)).collect::<Vec<_>>(),
		);

		// Budget and endow nominator stashes
		endowments.append(
			&mut self
				.nominators
				.iter()
				.map(|x| (x.clone(), PER_NOMINATOR_STASH))
				.collect::<Vec<_>>(),
		);

		// Budget and endow validator stashes
		endowments.append(
			&mut self.stakes.iter().map(|x| (x.clone(), PER_VALIDATOR_STASH)).collect::<Vec<_>>(),
		);

		// Load session keys to bootstrap validators from file
		let authorities: Vec<_> = self
			.bootstraps
			.iter()
			.enumerate()
			.map(|(i, x)| {
				(
					self.controller.clone().unwrap_or(self.stakes[i].clone()),
					self.stakes[i].clone(),
					timechain_runtime::SessionKeys {
						babe: x.0.clone(),
						grandpa: x.1.clone(),
						im_online: x.2.clone(),
						authority_discovery: x.3.clone(),
					},
				)
			})
			.collect();

		// Self-stake all authorities
		let locked = PER_VALIDATOR_STASH - PER_VALIDATOR_UNLOCKED;
		let stakers = authorities
			.iter()
			.map(|x| (x.1.clone(), x.0.clone(), locked, StakerStatus::<AccountId>::Validator))
			.collect::<Vec<_>>();

		let genesis_patch = serde_json::json!({
			"balances": {
				"balances": endowments,
			},
			"babe": {
				"epochConfig": timechain_runtime::BABE_GENESIS_EPOCH_CONFIG,
			},
			"elections": {
				"shardSize": shard_size,
				"shardThreshold": shard_threshold,
			},
			"networks": {
				"networks": [],
			},
			"session": {
				"keys": authorities,
			},
			"staking": {
				"validatorCount": authorities.len() as u32,
				"minimumValidatorCount": MIN_VALIDATOR_COUNT,
				"invulnerables": authorities.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
				"slashRewardFraction": Perbill::from_percent(10),
				"stakers": stakers
			},
			"technicalCommittee": {
				"members": Some(self.admins.clone()),
			},
		});

		// Put it all together ...
		let mut builder = ChainSpec::builder(wasm_binary, Default::default())
			.with_name(&name)
			.with_id(id)
			.with_protocol_id(id)
			.with_chain_type(chain_type)
			.with_properties(properties)
			.with_genesis_config_patch(genesis_patch);

		// ... and add optional telemetry
		if let Some(endpoints) = telemetry {
			builder = builder.with_telemetry_endpoints(endpoints);
		}

		// ... to generate chain spec
		Ok(builder.build())
	}
}
