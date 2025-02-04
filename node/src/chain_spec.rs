use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};

use polkadot_sdk::*;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::{json_merge, ChainSpecExtension};
use sc_service::{config::TelemetryEndpoints, ChainType};

use sp_authority_discovery::AuthorityId as DiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::UncheckedInto;
use sp_keyring::{AccountKeyring, Ed25519Keyring};

use time_primitives::{AccountId, Balance, Block, BlockNumber, ANLOG, SS58_PREFIX, TOKEN_DECIMALS};
use timechain_runtime::{RUNTIME_VARIANT, WASM_BINARY};

/// Stash and float for validators
const PER_VALIDATOR_STASH: Balance = ANLOG * 500_000;
const PER_VALIDATOR_UNLOCKED: Balance = ANLOG * 20_000;

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
	#[allow(dead_code)]
	chronicles: Vec<AccountId>,
	/// Optional controller account that will control all nominates stakes
	controller: Option<AccountId>,
	/// Additional endowed accounts and their balance in ANLOG.
	endowments: Vec<(AccountId, Balance)>,
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
			endowments: vec![],
			stakes: vec![
				Alice.into(),
				Bob.into(),
				Charlie.into(),
				Dave.into(),
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
	#[allow(dead_code)]
	pub fn to_live(&self) -> Result<ChainSpec, String> {
		assert!(
			!cfg!(feature = "develop"),
			"Runtimes with 'develop' feature are not safe to be used in production."
		);

		if cfg!(feature = "testnet") {
			self.to_chain_spec("analog-testnet", "TANLOG", ChainType::Live)
		} else {
			self.to_chain_spec("analog-timechain", "ANLOG", ChainType::Live)
		}
	}

	/// Generate development chain for supplied sub-identifier
	pub fn to_development(&self, subid: &str) -> Result<ChainSpec, String> {
		if !cfg!(feature = "develop") {
			log::warn!("Development chain with missmatched runtime variant: {}", RUNTIME_VARIANT);
		}

		let id = "analog-".to_owned() + subid;
		self.to_chain_spec(id.as_str(), "DANLOG", ChainType::Development)
	}

	/// Generate a local chain spec
	pub fn to_local(&self) -> Result<ChainSpec, String> {
		let id = "analog-".to_owned() + RUNTIME_VARIANT + "-local";
		self.to_chain_spec(id.as_str(), "LANLOG", ChainType::Local)
	}

	/// Generate a chain spec from key config
	fn to_chain_spec(
		&self,
		id: &str,
		token_symbol: &str,
		chain_type: ChainType,
	) -> Result<ChainSpec, String> {
		// Determine name from identifier
		let name = id.to_case(Case::Title);

		// Ensure wasm binary is available
		let wasm_binary = WASM_BINARY.expect(
			"The wasm runtime was not included with this release, i.e. the client was built with the \
			 `SKIP_WASM_BUILD` flag and it is only usable for chains that have already been initalized.\
			 Please rebuild with the flag disabled to start new chains from genesis.",
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

		// Endow chronicle stashes
		endowments.append(
			&mut self
				.chronicles
				.iter()
				.map(|x| (x.clone(), PER_CHRONICLE_STASH))
				.collect::<Vec<_>>(),
		);

		// Endow controller if necessary
		if let Some(controller) = self.controller.as_ref() {
			endowments.push((controller.clone(), CONTROLLER_SUPPLY));
		}

		// Endow council members and validators
		endowments.append(
			&mut self.admins.iter().map(|x| (x.clone(), PER_COUNCIL_STASH)).collect::<Vec<_>>(),
		);

		endowments.append(
			&mut self.stakes.iter().map(|x| (x.clone(), PER_VALIDATOR_STASH)).collect::<Vec<_>>(),
		);

		#[cfg(feature = "testnet")]
		// Currently still needed for GMP (6d6f646c70792f74727372790000000000000000000000000000000000000000)
		endowments.push((timechain_runtime::Treasury::account_id(), 20_000_000 * ANLOG));

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

		let mut genesis_patch = serde_json::json!({
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
		});

		if cfg!(feature = "testnet") {
			// Self-stake all authorities
			let locked = PER_VALIDATOR_STASH - PER_VALIDATOR_UNLOCKED;
			let stakers = authorities
				.iter()
				.map(|x| {
					(
						x.1.clone(),
						x.0.clone(),
						locked,
						timechain_runtime::StakerStatus::<AccountId>::Validator,
					)
				})
				.collect::<Vec<_>>();

			json_merge(
				&mut genesis_patch,
				serde_json::json!({
					"networks": {
						"networks": [],
					},
					"staking": {
						"validatorCount": authorities.len() as u32,
						"minimumValidatorCount": MIN_VALIDATOR_COUNT,
						"invulnerables": authorities.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
						"slashRewardFraction": sp_runtime::Perbill::from_percent(10),
						"stakers": stakers
					},
				}),
			);
		}

		if cfg!(feature = "develop") {
			use AccountKeyring::*;

			let airdrop: Vec<(AccountId, Balance)> =
				vec![(One.into(), 10_000 * ANLOG), (Two.into(), 10_000 * ANLOG)];

			let vesting: Vec<(AccountId, Balance, Balance, BlockNumber)> =
				vec![(Two.into(), 8_000 * ANLOG, 80 * ANLOG, 100)];

			json_merge(
				&mut genesis_patch,
				serde_json::json!({
					"airdrop": {
						"claims": airdrop,
						"vesting": vesting,
					}
				}),
			);
		}

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
