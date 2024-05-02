use convert_case::{Case, Casing};
use hex_literal::hex;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_service::{config::TelemetryEndpoints, ChainType};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::UncheckedInto, hex2array};
use sp_keyring::{AccountKeyring, Ed25519Keyring};
use sp_runtime::Perbill;
use timechain_runtime::{
	AccountId, Balance, RuntimeGenesisConfig as GenesisConfig, StakerStatus, ANLOG, TOKEN_DECIMALS,
	WASM_BINARY,
};
const SS_58_FORMAT: u32 = 12850;

/// Total supply of token is 90_570_710.
/// Initially we are distributing the total supply to the multiple accounts which is representing
/// its category pool which we will update in later part of development.
const SEED_ROUND_SUPPLY: Balance = ANLOG * 24_275_364;
const INITIAL_PRIVATE_SALE: Balance = ANLOG * 1_837_476;
const PRIVATE_SALE: Balance = ANLOG * 8_919_012;
const PUBLIC_SALE: Balance = ANLOG * 1_449_275;
const TEAM_SUPPLY: Balance = ANLOG * 17_210_160;
const TREASURY_SUPPLY: Balance = ANLOG * 13_224_636;
const COMMUNITY_SUPPLY: Balance = ANLOG * 23_663_800;

/// Stash and float for validators
const PER_VALIDATOR_STASH: Balance = ANLOG * 500_000;
const PER_VALIDATOR_UNLOCKED: Balance = ANLOG * 20_000;

/// Stash for community nominations
const PER_NOMINATION: Balance = ANLOG * 180_000;
const PER_NOMINATOR_STASH: Balance = 8 * PER_NOMINATION;

/// Stash and float for chronicles
const PER_CHRONICLE_STASH: Balance = ANLOG * 100_000;

/// Token supply for prefunded admin accounts
const SUDO_SUPPLY: Balance = ANLOG * 50_000;
const CONTROLLER_SUPPLY: Balance = ANLOG * 50_000;
const PER_COUNCIL_STASH: Balance = ANLOG * 50_000;

/// Minimum needed validators, currently lowered for testing environments
const MIN_VALIDATOR_COUNT: u32 = 1;

/// Default telemetry server for all networks
const DEFAULT_TELEMETRY_URL: &str = "wss://telemetry.analog.one/submit";
const DEFAULT_TELEMETRY_LEVEL: u8 = 1;

/// Additional development keys used for chronicles
const THREE: [u8; 32] =
	hex2array!("9026941b7aa2328a8c5ea4e25bb747a2bf92a066fae0cc3722faf58cf44d3502");
const FOUR: [u8; 32] =
	hex2array!("4017e17f10cc5a98731de9f020dbb37986f6e575789152d7fadae2b32eea6c13");
const FIVE: [u8; 32] =
	hex2array!("b0521e374b0586d6829dad320753c62cdc6ef5edbd37ffdd36da0ae97c521819");
const SIX: [u8; 32] =
	hex2array!("1880104772db7b947f3f8ccdcab3650d7179c44551d22dd0cca5dc852a140563");

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper to parse genesis keys json
#[derive(serde::Deserialize)]
pub struct GenesisKeysConfig {
	/// Keys used to bootstrap validator session keys.
	/// Will match and register session keys to stashes and self-stake them.
	/// Balance to be staked is controlled by PER_VALIDATOR_UNLOCKED
	bootstraps: Vec<(BabeId, GrandpaId, ImOnlineId)>,
	/// Stashes to be used for chronicles, balances controlled by PER_CHRONICLE_STASH
	chronicles: Vec<AccountId>,
	/// Optional controller account that will control all nominates stakes
	controller: Option<AccountId>,
	/// Genesis members of on-chain council
	councils: Vec<AccountId>,
	/// Additional endowed accounts and their balance in ANLOG.
	endowments: Vec<(AccountId, Balance)>,
	/// Stashes intended for community nominations.
	/// Sizing controlled by PER_NOMINATION_STASH
	nominators: Vec<AccountId>,
	/// Stashes intended to be used to run validators.
	/// There has to be at least one stash for every
	/// session key set. Balance controlled by PER_VALIDATOR_STASH
	stakes: Vec<AccountId>,
	/// Root account to controll sudo pallet
	sudo: AccountId,
}

impl Default for GenesisKeysConfig {
	/// Default configuration using know development keys
	fn default() -> Self {
		use AccountKeyring::*;

		GenesisKeysConfig {
			bootstraps: vec![
				(
					Alice.to_raw_public().unchecked_into(),
					Ed25519Keyring::Alice.to_raw_public().unchecked_into(),
					Alice.to_raw_public().unchecked_into(),
				),
				(
					Bob.to_raw_public().unchecked_into(),
					Ed25519Keyring::Bob.to_raw_public().unchecked_into(),
					Bob.to_raw_public().unchecked_into(),
				),
			],
			chronicles: vec![
				One.into(),
				Two.into(),
				THREE.into(),
				FOUR.into(),
				FIVE.into(),
				SIX.into(),
			],
			// TODO: Would be better to assign individual controllers
			controller: None,
			councils: vec![Bob.into(), Charlie.into(), Dave.into(), Eve.into(), Ferdie.into()],
			endowments: vec![],
			nominators: vec![],
			stakes: vec![
				AliceStash.into(),
				BobStash.into(),
				CharlieStash.into(),
				DaveStash.into(),
				EveStash.into(),
				FerdieStash.into(),
			],
			sudo: Alice.into(),
		}
	}
}

impl GenesisKeysConfig {
	/// Deserialize genesis key config from json bytes
	pub fn from_json_bytes(json: &[u8]) -> Result<Self, String> {
		serde_json::from_slice(json).map_err(|e| e.to_string())
	}

	/// Generate development chain for supplied sub-identifier
	pub fn to_development_spec(&self, id: &str) -> Result<ChainSpec, String> {
		let id = "analog_".to_owned() + id;
		self.to_chain_spec(id.as_str(), ChainType::Development, 6, 4)
	}

	/// Generate a local chain spec with the supplied shard size and threshold
	pub fn to_local_spec(
		&self,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<ChainSpec, String> {
		self.to_chain_spec("analog_local", ChainType::Local, shard_size, shard_threshold)
	}

	/// Generate a chain spec from key config
	fn to_chain_spec(
		&self,
		id: &str,
		chain_type: ChainType,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<ChainSpec, String> {
		let wasm_binary =
			WASM_BINARY.ok_or_else(|| "Analog wasm runtime not available".to_string())?;

		// Determine name from identifier
		let name = id.to_case(Case::Title);

		// Determine token symbol based on chain type
		let token_symbol = match chain_type {
			ChainType::Development => "DANLOG",
			ChainType::Local => "LANLOG",
			_ => return Err("Unsupported chain type".to_string()),
		};

		// Setup base currency unit name and decimal places
		let mut properties = sc_chain_spec::Properties::new();
		properties.insert("tokenSymbol".into(), token_symbol.into());
		properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
		properties.insert("ss58Format".into(), SS_58_FORMAT.into());

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
		let chronicle_supply = self.chronicles.len() as u128 * PER_CHRONICLE_STASH;
		endowments.append(
			&mut self
				.chronicles
				.iter()
				.map(|x| (x.clone(), PER_CHRONICLE_STASH))
				.collect::<Vec<_>>(),
		);

		// Endow controller if necessary
		let mut controller_supply = 0u128;
		if let Some(controller) = self.controller.as_ref() {
			controller_supply = CONTROLLER_SUPPLY;
			endowments.append(&mut vec![(controller.clone(), CONTROLLER_SUPPLY)]);
		}

		// Budget and endow council stashes
		let council_supply = self.councils.len() as u128 * PER_COUNCIL_STASH;
		endowments.append(
			&mut self.councils.iter().map(|x| (x.clone(), PER_COUNCIL_STASH)).collect::<Vec<_>>(),
		);

		// Budget and endow nominator stashes
		let nominator_supply = self.nominators.len() as u128 * PER_NOMINATOR_STASH;
		endowments.append(
			&mut self
				.nominators
				.iter()
				.map(|x| (x.clone(), PER_NOMINATOR_STASH))
				.collect::<Vec<_>>(),
		);

		// Budget and endow validator stashes
		let stake_supply = self.stakes.len() as u128 * PER_VALIDATOR_STASH;
		endowments.append(
			&mut self.stakes.iter().map(|x| (x.clone(), PER_VALIDATOR_STASH)).collect::<Vec<_>>(),
		);

		// Endow sudo account
		endowments.append(&mut vec![(self.sudo.clone(), SUDO_SUPPLY)]);

		// Add simulated supplies
		endowments.append(&mut vec![
			(
				hex!["0062466de473bc2686173eed44f49b282bf1615f4287ce8566aeaa5747a70855"].into(),
				SEED_ROUND_SUPPLY,
			),
			(
				hex!["5e489fd2dfc7dceb07c2f767d3e81928378330c2cef4dd58eb184582cc56d649"].into(),
				INITIAL_PRIVATE_SALE,
			),
			(
				hex!["1645738c66053277fdbcf04631805a7392ce23b043dc60862d8af09a329f0a79"].into(),
				PRIVATE_SALE,
			),
			(
				hex!["588de6ea1b423e0fc41995525a1fd63f50ec1e0c0b9bcc8192eb766eb85fce2f"].into(),
				PUBLIC_SALE,
			),
			(
				hex!["62e926d7df56786c766af140cdc9da839c50e60fa0d6722488a1ad235f1c5d1a"].into(),
				TEAM_SUPPLY - SUDO_SUPPLY - controller_supply - council_supply,
			),
			(
				hex!["ca6b881965b230aa52153c972ca0dc3dd0fa0a7453c00b62dec3532716fcd92d"].into(),
				TREASURY_SUPPLY,
			),
			(
				hex!["f612a8386a524dc0159463e5b2d01624d1730603fac6a5a1191aa32569138c4c"].into(),
				COMMUNITY_SUPPLY - stake_supply - nominator_supply - chronicle_supply,
			),
		]);

		// Load session keys to bootstrap validators from file
		let authorities: Vec<_> = self
			.bootstraps
			.iter()
			.enumerate()
			.map(|(i, x)| {
				(
					self.controller.clone().unwrap_or(self.stakes[i].clone()),
					self.stakes[i].clone(),
					timechain_runtime::opaque::SessionKeys {
						babe: x.0.clone(),
						grandpa: x.1.clone(),
						im_online: x.2.clone(),
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
				"networks": [
					("ethereum", "mainnet"),
					("astar", "astar"),
					("polygon", "mainnet"),
					("ethereum", "dev"),
					("ethereum", "goerli"),
					("ethereum", "sepolia"),
					("astar", "dev"),
				],
			},
			"sudo": {
				"key": Some(self.sudo.clone()),
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
			"council": {
				"members": self.councils,
			},
		});

		// Put it all together ...
		let mut builder = ChainSpec::builder(wasm_binary, None)
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
