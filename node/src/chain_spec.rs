use hex_literal::hex;
use runtime_common::constants::{Balance, ANLOG, TOKEN_DECIMALS};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use timechain_runtime::{
	AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig,
	SystemConfig, WASM_BINARY,
};

const TOKEN_SYMBOL: &str = "ANLOG";
const SS_58_FORMAT: u32 = 51;

/// Total supply of token is 90_570_710.
/// Initially we are distributing the total supply to the multiple accounts which is representing
/// its category pool which we will update in later part of development.
const SEED_ROUND_SUPPLY: Balance = ANLOG * 24_275_362;
const TEAM_SUPPLY: Balance = ANLOG * 17_210_144;
const PUBLIC_SALE_SUPPLY: Balance = ANLOG * 1_449_275;
const TREASURY_SUPPLY: Balance = ANLOG * 13_224_637;
const COMMUNITY_SUPPLY: Balance = ANLOG * 27_173_913;
const DEVELOPER_SUPPLY: Balance = ANLOG * 4_528_989;
const BOUNTY_PROGRAM_SUPPLY: Balance = ANLOG * 2_717_391;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

/// Generate a default spec for the Analog live chain (Prod).
pub fn analog_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Analog live wasm not available".to_string())?;

	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Analog Live",
		// ID
		"prod",
		ChainType::Live,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					(
						hex!["88fd77d706e168d78713a6a927c1ddfae367b081fb2829b119bbcc6db9af401d"]
							.into(),
						SEED_ROUND_SUPPLY,
					),
					(
						hex!["04063fc1cbba917ced6c45091bf631de6a4db584dd55c1d67431661a5d57a575"]
							.into(),
						TEAM_SUPPLY,
					),
					(
						hex!["b2e1b641dad4cac54938a00db749e135e95a4781cf4bc52aaac7b833fbb5cd0a"]
							.into(),
						PUBLIC_SALE_SUPPLY,
					),
					(
						hex!["cc5245e57dcf6c8f051e012beceaa1683578ae873223d3ef4f8cbd85a62e1536"]
							.into(),
						TREASURY_SUPPLY,
					),
					(
						hex!["2af7c08133177cc462171389578174b89758ca09c5f93235409594f15f65ac63"]
							.into(),
						COMMUNITY_SUPPLY,
					),
					(
						hex!["f6855b0ec40cc91c49025d75aa65a1965861cde56451da99170bd4dae13dab35"]
							.into(),
						DEVELOPER_SUPPLY,
					),
					(
						hex!["e0dc12faf7e650b910638e934b4ef9aea1410707312bd8d80ec91123acb02747"]
							.into(),
						BOUNTY_PROGRAM_SUPPLY,
					),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

/// Generate a chain spec for Analog development chain.
pub fn analog_development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					(
						hex!["88fd77d706e168d78713a6a927c1ddfae367b081fb2829b119bbcc6db9af401d"]
							.into(),
						SEED_ROUND_SUPPLY,
					),
					(
						hex!["04063fc1cbba917ced6c45091bf631de6a4db584dd55c1d67431661a5d57a575"]
							.into(),
						TEAM_SUPPLY,
					),
					(
						hex!["b2e1b641dad4cac54938a00db749e135e95a4781cf4bc52aaac7b833fbb5cd0a"]
							.into(),
						PUBLIC_SALE_SUPPLY,
					),
					(
						hex!["cc5245e57dcf6c8f051e012beceaa1683578ae873223d3ef4f8cbd85a62e1536"]
							.into(),
						TREASURY_SUPPLY,
					),
					(
						hex!["2af7c08133177cc462171389578174b89758ca09c5f93235409594f15f65ac63"]
							.into(),
						COMMUNITY_SUPPLY,
					),
					(
						hex!["f6855b0ec40cc91c49025d75aa65a1965861cde56451da99170bd4dae13dab35"]
							.into(),
						DEVELOPER_SUPPLY,
					),
					(
						hex!["e0dc12faf7e650b910638e934b4ef9aea1410707312bd8d80ec91123acb02747"]
							.into(),
						BOUNTY_PROGRAM_SUPPLY,
					),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

/// Generate a chain spec for Analog testnet chain.
pub fn analog_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), 2000000),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), 1000000),
					(get_account_id_from_seed::<sr25519::Public>("Charlie"), 1000000),
					(get_account_id_from_seed::<sr25519::Public>("Dave"), 10000000),
					(get_account_id_from_seed::<sr25519::Public>("Eve"), 1000000),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 1000000),
					(get_account_id_from_seed::<sr25519::Public>("Alice//stash"), 1000000),
					(get_account_id_from_seed::<sr25519::Public>("Bob//stash"), 10000000),
					(get_account_id_from_seed::<sr25519::Public>("Charlie//stash"), 1000000),
					(get_account_id_from_seed::<sr25519::Public>("Dave//stash"), 1000),
					(get_account_id_from_seed::<sr25519::Public>("Eve//stash"), 1000),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"), 1000),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	_enable_println: bool,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			// Configure pool accounts with its initial supply.
			balances: endowed_accounts,
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
	}
}
