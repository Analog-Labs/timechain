use hex_literal::hex;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_service::ChainType;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::UncheckedInto;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
use timechain_runtime::{
	AccountId, Balance, BalancesConfig, CouncilConfig, ElectionsConfig, GrandpaConfig,
	ImOnlineConfig, RuntimeGenesisConfig as GenesisConfig, Signature, StakerStatus, StakingConfig,
	SudoConfig, SystemConfig, ANLOG, SHARD_SIZE, SHARD_THRESHOLD, TOKEN_DECIMALS, WASM_BINARY,
};
const TOKEN_SYMBOL: &str = "TANLOG";
const SS_58_FORMAT: u32 = 51;

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
const PER_VALIDATOR_STASH: Balance = ANLOG * 500000;
const PER_VALIDATOR_UNLOCKED: Balance = ANLOG * 20000;

/// Stash for community nominations
const PER_NOMINATION: Balance = ANLOG * 180000;
const PER_NOMINATOR_STASH: Balance = 8 * PER_NOMINATION;

/// Stash and float for chronicles
const PER_CHRONICLE_STASH: Balance = ANLOG * 100000;

/// Token supply for prefunded admin accounts
const SUDO_SUPPLY: Balance = ANLOG * 50000;
const CONTROLLER_SUPPLY: Balance = ANLOG * 50000;

/// Token supply for prefunded dev team test user account
const TESTUSER_SUPPLY: Balance = ANLOG * 100000;

/// Deprecated supply constants for dev envs
const VALIDATOR_SUPPLY: Balance = 10 * PER_VALIDATOR_STASH;
const CHRONICLE_SUPPLY: Balance = 12 * PER_CHRONICLE_STASH;

/// Minimum needed validators, currently lowered to improve stability
const MIN_VALIDATOR_COUNT: u32 = 4;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{seed}"), None)
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
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
	(
		get_account_id_from_seed::<sr25519::Public>(s),
		get_account_id_from_seed::<sr25519::Public>(&format!("{s}//stash")),
		get_from_seed::<BabeId>(s),
		get_from_seed::<GrandpaId>(s),
		get_from_seed::<ImOnlineId>(s),
	)
}

/// Helper to parse hex public keys
#[serde_with::serde_as]
#[derive(serde::Deserialize)]
struct AccountHex(#[serde_as(as = "serde_with::hex::Hex")] [u8; 32]);

/// Generate a chain spec for testnet deployment
pub fn analog_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Analog Testnet wasm not available".to_string())?;

	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	// Operator keys
	let sudo_account: AccountId =
		hex!["ce011c3cde3175fdfb952f243642d20ab3c9effb876b2552f19d254ba13ea252"].into();
	let controller_account: AccountId =
		hex!["ec91a0de241bd290d60d688c839a50b71ddf13f5d43bebf664eb076d12ba513b"].into();

	// Load validator stashes from file
	let stakes_json = &include_bytes!("genesis/testnet.stakes.json")[..];
	let stake_keys: Vec<AccountHex> =
		serde_json::from_slice(stakes_json).expect("Missing or invalid stake keys");
	let validator_supply = stake_keys.len() as u128 * PER_VALIDATOR_STASH;
	let validator_accounts: Vec<_> =
		stake_keys.iter().map(|x| (x.0.into(), PER_VALIDATOR_STASH)).collect();

	// Load nomination stashes from file
	let nominators_json = &include_bytes!("genesis/testnet.nominators.json")[..];
	let nominator_keys: Vec<AccountHex> =
		serde_json::from_slice(nominators_json).expect("Missing or invalid nomination keys");
	let nominator_supply = nominator_keys.len() as u128 * PER_NOMINATOR_STASH;
	let nominator_accounts: Vec<_> =
		nominator_keys.iter().map(|x| (x.0.into(), PER_NOMINATOR_STASH)).collect();

	// Load chronicle stashes from file
	let chronicles_json = &include_bytes!("genesis/testnet.chronicles.json")[..];
	let chronicle_keys: Vec<AccountHex> =
		serde_json::from_slice(chronicles_json).expect("Missing or invalid chronicle keys");
	let chronicle_supply = chronicle_keys.len() as u128 * PER_CHRONICLE_STASH;
	let chronicle_accounts: Vec<_> =
		chronicle_keys.iter().map(|x| (x.0.into(), PER_CHRONICLE_STASH)).collect();

	// Load session keys to bootstrap validators from file
	let bootstrap_json = &include_bytes!("genesis/testnet.bootstrap.json")[..];
	let bootstrap_keys: Vec<(AccountHex, AccountHex, AccountHex)> =
		serde_json::from_slice(bootstrap_json).expect("Missing or invalid bootstrap keys");
	let authorities = bootstrap_keys
		.into_iter()
		.enumerate()
		.map(|(i, x)| {
			(
				controller_account.clone(),
				stake_keys[i].0.into(),
				x.0 .0.unchecked_into(),
				x.1 .0.unchecked_into(),
				x.2 .0.unchecked_into(),
			)
		})
		.collect();

	// Put it all together to generate chain spec
	Ok(ChainSpec::builder(wasm_binary, None)
	   .with_name("Analog Testnet")
	   .with_id("anlogcc1")
	   .with_protocol_id("anlogcc1")
	   .with_chain_type(ChainType::Live)
	   .with_properties(properties)
	   .with_boot_nodes(vec![
		   "/dns/bootnode-1.testnet.analog.one/tcp/30333/ws/p2p/12D3KooWCBYeyrxGref7xs6s5W2hDu8po8GG4B8ApxqR6RG8RRW5".parse().unwrap(),
		   "/dns/bootnode-2.testnet.analog.one/tcp/30334/ws/p2p/12D3KooWQcuGD4sqcqPX2G8DcZpmSaLgnF5NjeCxf9NCL2oof5Ec".parse().unwrap(),
	   ])
	   .with_genesis_config_patch(analog_genesis_json(
		   // Sudo account
		   sudo_account.clone(),
		   // Council account
		   hex!["143cda6b33902c40050d7b85b9393f5db16eeafe1f728d50ac57404c85442a10"].into(),
		   // Initial authorities at genesis
		   authorities,
	 	  // Pre-funded accounts
		   vec![
				// Sudo stashes
				(
					sudo_account,
					SUDO_SUPPLY
				),
				// Controller stashes
				(
					controller_account,
					CONTROLLER_SUPPLY
				),

				// Test user stashes
				(
					hex!["4c2dc62c56bd4c333de73b70df184e4a2dd982762675b23663abdc66d4030350"]
						.into(),
					TESTUSER_SUPPLY
				),

				// Tokenomics and supply
				(
					hex!["0062466de473bc2686173eed44f49b282bf1615f4287ce8566aeaa5747a70855"]
						.into(),
					SEED_ROUND_SUPPLY,
				),
				(
					hex!["5e489fd2dfc7dceb07c2f767d3e81928378330c2cef4dd58eb184582cc56d649"]
						.into(),
					INITIAL_PRIVATE_SALE,
				),
				(
					hex!["1645738c66053277fdbcf04631805a7392ce23b043dc60862d8af09a329f0a79"]
						.into(),
					PRIVATE_SALE,
				),
				(
					hex!["588de6ea1b423e0fc41995525a1fd63f50ec1e0c0b9bcc8192eb766eb85fce2f"]
						.into(),
					PUBLIC_SALE,
				),
				(
					hex!["62e926d7df56786c766af140cdc9da839c50e60fa0d6722488a1ad235f1c5d1a"]
						.into(),
					TEAM_SUPPLY - SUDO_SUPPLY - CONTROLLER_SUPPLY - TESTUSER_SUPPLY - chronicle_supply,
				),
				(
					hex!["ca6b881965b230aa52153c972ca0dc3dd0fa0a7453c00b62dec3532716fcd92d"]
						.into(),
					TREASURY_SUPPLY,
				),
				(
					hex!["f612a8386a524dc0159463e5b2d01624d1730603fac6a5a1191aa32569138c4c"]
						.into(),
					COMMUNITY_SUPPLY - validator_supply - nominator_supply,
				),
			].into_iter()
			 .chain(validator_accounts)
			 .chain(nominator_accounts)
			 .chain(chronicle_accounts)
			 .collect(),
	   ))
	   .build()
	)
}

/// Generate a chain spec for Analog staging environment.
pub fn analog_staging_config(disable_tss: bool) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Staging wasm not available".to_string())?;

	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	#[allow(deprecated)]
	Ok(ChainSpec::from_genesis(
		// Name
		"Analog Staging",
		// ID
		"analog_staging",
		ChainType::Development,
		move || {
			generate_analog_genesis(
				// Sudo account
				hex!["166ce0ffbe439609d59ab5aec79c00f4d7da021b856ccb412510f75791cf0a7d"].into(),
				// Council account
				hex!["a07e62ca85f6d68d83f3a6924bc7a47c6ad0a6ea1f43d7b59407af268d39fb43"].into(),
				// Initial authorities at genesis
				vec![
					// node 0
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["720e6fc7dafc68294f7f7a971dc0956b048a5913cd2c10dd748fe64016c3da04"].into(),

						hex!["d0886931cc61c1468ca568f375b00ae342d524c9161de8ebafb45c79c604696d"].unchecked_into(),
						hex!["ef4bcee93fecb0a60d204199fcef2806c69656dda95dfac031d104b8d0f550e8"].unchecked_into(),
						hex!["c86d88487b6ff18cff1d9f1575e201edbc36988a4f265829f783d7b902431a01"].unchecked_into(),
					),
					// node 1
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["227a28579209bf5f5f150ff88646fda57fd4283d172f19684ef609677925bb59"].into(),

						hex!["aadc6e4eac1fe82120401fcd156cc3e461182476a560e558dff56febdfc7a825"].unchecked_into(),
						hex!["93cbb94d6afd4fdd83b08cd4219c5d857ca25126cb29391a250a81c8a74f4d59"].unchecked_into(),
						hex!["c8b0e928ec3c34bf749b9968268039ee2d90b9f797fa879e0fc011b17b38ab22"].unchecked_into(),
					),
					// node 2
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["44087eacb3fa5576cb4ece5ddde4417024fcfe0e444ebe4061565eaebcb2a35a"].into(),

						hex!["f25d34df509f86c4c804ca3f696c5a75d42587568d87a48f771aff3a5fcbb023"].unchecked_into(),
						hex!["5a731f7a61a1405d647feaeec607c915bd2202426ba07615d6b8787f6283d149"].unchecked_into(),
						hex!["10974d84d7d69fd775e270d8b7d419281a0d07bc647dd9de24c69201f0d55e53"].unchecked_into(),
					),
					// node 3
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["b83e55e19c6452462a452e1895ca8de155172840d70c0937dedab109efb9ef08"].into(),

						hex!["5e2d60a3a2337039b9e58ce50fecc991cd0ea7673a2387dbb37b287494ac2b60"].unchecked_into(),
						hex!["f35b5fb1554b512c51e101bdd592fac3e439de15c6f65133da0cbccbc71cf674"].unchecked_into(),
						hex!["3c3b548ed31970d5a36b98e0fa84fbe980ca75c92829678c9bf6935ca162ff5c"].unchecked_into(),
					),
					// node 4
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["34788c986a9d9236b572c762ebc47fdd32afd0116d4516c45b9ab655eb0f9c56"].into(),

						hex!["ba20b15a4abb8287994a9c1976aceb1b21464f23280a35db068d0515dd457706"].unchecked_into(),
						hex!["235be4117640fb7d164b17bbb758e81205f6583db17eb4927ce9d3d599a87e94"].unchecked_into(),
						hex!["1efa3587920e6acf6108bcc99ea0e2f7cf7a29752fb8b5d72cf3f9f9884e930a"].unchecked_into(),
					),
					// node 5
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["5cac7f9e4efbc6a09b40b1a5545fb8ad54b6951f1618020ec43801329116c457"].into(),

						hex!["3cac8e3b950618f9896f9ffd71768ac1d16ea0e080fac7ff5c723f68095eb876"].unchecked_into(),
						hex!["b2fbffa464546473a51b890165c17ce438eaee21ed4ecf6499bacfd2fa7a6b96"].unchecked_into(),
						hex!["400d522da7b25ac6a63414c9c217f0786e52a3123a37996bb21a6d261fdbec79"].unchecked_into(),
					),
					// node 6
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["1821c25233213e966f485a1ea9a09e865d857c26cc8c0710ed55030762876202"].into(),

						hex!["0a15a77a4b32f4928ca5fc07731f5f1a6bef0141c067a1447a85813871ac3875"].unchecked_into(),
						hex!["db50891efee43400735a827e1a28482f264d3634138f7acd81b79daa7548791a"].unchecked_into(),
						hex!["aa2318f2f679dd2b7d78d474f09b0a459ee18a8d1dc6503a56efe0ccff45c304"].unchecked_into(),
					),
					// node 7
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["4857f11bcb3e575dbd5cb71e1aa523c57e9e5b6acd33def8347226357174f638"].into(),

						hex!["661a923d91c0616102c6545c9ac79a250541dd4606d17ad1eca6c9530e6d6a2c"].unchecked_into(),
						hex!["3753e40d5b2fa6bb80991632750f6c65ff53adcb3775b36baa0870bf1a22465e"].unchecked_into(),
						hex!["f621e42e02cae2e3a087d50e0ca7fbeffd27600ac8f2f74a82c9414be00a6e3d"].unchecked_into(),
					),
					// node 8
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["76fe80fd6b807ddc1af90634236ab0d909f11505198101dbb569d563ada0b722"].into(),

						hex!["aaac4691db35c6cefd040aef7ebbbabe6feb9641eea05185d85406f53261db06"].unchecked_into(),
						hex!["460bcaed9cc60b630a8201a85184358276cd172d40c5011c58db37fad7205d88"].unchecked_into(),
						hex!["fec9a79f0075996812d76ac399e72985531a495347a98b5811a42dbf0e325435"].unchecked_into(),
					),
					// node 9
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"].into(),
						hex!["a810e46c42031e701c4ab22879bec5efada9ff957a9bf2a0763907cc02172a1b"].into(),

						hex!["50ac10bb70a001d7cb83a7f921aafdc2810a28df68ee0735ef12021bbf11135b"].unchecked_into(),
						hex!["cafe9fe1a583b7a086afcf7e66ec5ea97457dd23fdd126b5ddfb9d39310c6434"].unchecked_into(),
						hex!["1e2c106b946396d63e156b10f5cbbcc7f0d0d7813825b55bf82517ac60496138"].unchecked_into(),
					),
				],
				// Pre-funded accounts
				vec![
					// Sudo stashes
					(
						hex!["166ce0ffbe439609d59ab5aec79c00f4d7da021b856ccb412510f75791cf0a7d"]
							.into(),
						SUDO_SUPPLY
					),
					// Controller stashes
					(
						hex!["ced65a5c8089791384cfa2e92825744b77622f2f32267614e864f9ba65f5135f"]
							.into(),
						CONTROLLER_SUPPLY
					),

					// Test user stashes
					(
						hex!["10e467cff36a7ae9059963530db543ae07875dd3d61f9ea66334a43119b2d73e"]
							.into(),
						TESTUSER_SUPPLY
					),

					// Validator stashes
					(
						hex!["720e6fc7dafc68294f7f7a971dc0956b048a5913cd2c10dd748fe64016c3da04"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["227a28579209bf5f5f150ff88646fda57fd4283d172f19684ef609677925bb59"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["44087eacb3fa5576cb4ece5ddde4417024fcfe0e444ebe4061565eaebcb2a35a"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["b83e55e19c6452462a452e1895ca8de155172840d70c0937dedab109efb9ef08"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["34788c986a9d9236b572c762ebc47fdd32afd0116d4516c45b9ab655eb0f9c56"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["5cac7f9e4efbc6a09b40b1a5545fb8ad54b6951f1618020ec43801329116c457"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["1821c25233213e966f485a1ea9a09e865d857c26cc8c0710ed55030762876202"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["4857f11bcb3e575dbd5cb71e1aa523c57e9e5b6acd33def8347226357174f638"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["76fe80fd6b807ddc1af90634236ab0d909f11505198101dbb569d563ada0b722"]
							.into(),
						PER_VALIDATOR_STASH
					),
					(
						hex!["a810e46c42031e701c4ab22879bec5efada9ff957a9bf2a0763907cc02172a1b"]
							.into(),
						PER_VALIDATOR_STASH
					),

					// Chronicle account
					(
						hex!["d863c016582aeac712e14871f86ca402c129c99639a2b443ec1d5b4980945a7b"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["7aa4684cfe708788c61024fec3c40c3d2a91e978f29454fb6671c7b6f0c7a86d"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["1868e9232bd2cac078aecf2591e88c134ffed012a227e0eee749e03eed321518"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["8426681de5bffff3f13c3a7e9c3cd2c8e0d9d3851d64a5a159dcb44bf7c07430"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["72e8384fc6fdd083387b6a526706712ccee1912b07c8a481cb5f4c79bafaec4d"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["a68ed4a9449abf15a66233953f13beaf377c6e9b2d5750dc662be0c0132ce901"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["4e17b80ce9fc678cee1f3b5705fefff0847f093d65af838aafc47c7785409c0e"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["4a133a460ec1f5b42baf6ebc9e66433e3d8b96ffa9a860ea1cd152338cdf0403"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["8063b6e00240d67994ae972678e006d6c6bb1f52accb0036f7eaffa610d09219"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["16a6992d5e0e0792b6af63654333b1c7ee0868d715e0c8cef198965713ea1b0c"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["629e4705affcb9a38aee0d25af0bee848dc914b99d71817827f8bb2944b3d350"]
							.into(),
						PER_CHRONICLE_STASH
					),
					(
						hex!["001b3f53b227672d413acb61e49c8de70b82f68632a18ed6b85ed704bf994919"]
							.into(),
						PER_CHRONICLE_STASH
					),

					// Tokenomics and supply
					(
						hex!["28fb5fcce7c06f9aff08b55cdcfb8bd8131e74d86333abbe5ad17a308d9e9a62"]
							.into(),
						SEED_ROUND_SUPPLY,
					),
					(
						hex!["8064fb279fa7ff7115bdeb08285f18dad58759c68d7ab667b759478d53fcfb40"]
							.into(),
						INITIAL_PRIVATE_SALE,
					),
					(
						hex!["184e8fff719146fcf4d491f920f6124d61f4ffa4f3674c1c3384020512735d27"]
							.into(),
						PRIVATE_SALE,
					),
					(
						hex!["b2cefd2751550fe92f345fcc50a891f7d23c6fca7a70b7824743a4b4a0acf65a"]
							.into(),
						PUBLIC_SALE,
					),
					(
						hex!["a894a617e4cae275c9982f4b1777ede374d38f42f55b4d6f48fe647e55494e22"]
							.into(),
						TEAM_SUPPLY - SUDO_SUPPLY - CONTROLLER_SUPPLY - TESTUSER_SUPPLY - VALIDATOR_SUPPLY - CHRONICLE_SUPPLY,
					),
					(
						hex!["cc382667871c8eac8ab337058361d3e6f8c7d04990e4a2c9a4024993e502b418"]
							.into(),
						TREASURY_SUPPLY,
					),
					(
						hex!["c2a19463d52bb9a6aadb1e38e45817850a3444902e519213310915ebcbbbb65f"]
							.into(),
						COMMUNITY_SUPPLY,
					),
				],
				disable_tss,
			)
		},
		// Bootnodes
		vec![
			"/dns/bootnode-1.staging.analog.one/tcp/30333/ws/p2p/12D3KooWT3K83HvytjS5fzkssX2r1E86mQUjsVLiAeH3PkV1RQ1K".parse().unwrap(),
		],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
		wasm_binary,
	))
}

/// Generate a chain spec for local developement and testing.
pub fn analog_dev_config(disable_tss: bool) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	#[allow(deprecated)]
	Ok(ChainSpec::from_genesis(
		// Name
		"Analog Dev",
		// ID
		"analog_dev",
		ChainType::Development,
		move || {
			generate_analog_genesis(
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Council account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Pre-funded accounts
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), ANLOG * 2000000),
					(get_account_id_from_seed::<sr25519::Public>("Alice//stash"), ANLOG * 2000000),
					// dev prefund set-keys1, shard1
					(
						hex!["78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d"]
							.into(),
						ANLOG * 2000000,
					),
					// dev prefund set-keys2, shard1
					(
						hex!["cee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b"]
							.into(),
						ANLOG * 2000000,
					),
					// dev prefund set-keys3, shard1
					(
						hex!["a01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02"]
							.into(),
						ANLOG * 2000000,
					),
					// dev prefund set-keys4, shard2
					(
						hex!["1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f"]
							.into(),
						ANLOG * 2000000,
					),
					// dev prefund set-keys5, shard2
					(
						hex!["1843caba7078a699217b23bcec8b57db996fc3d1804948e9ee159fc1dc9b8659"]
							.into(),
						ANLOG * 2000000,
					),
					// dev prefund set-keys6, shard2
					(
						hex!["72a170526bb41438d918a9827834c38aff8571bfe9203e38b7a6fd93ecf70d69"]
							.into(),
						ANLOG * 2000000,
					),
					// dev prefund valiator account
					(
						hex!["862b57a754ebda4c4bbd5714b637becd83f868ff634df6c22d4a9a905596f911"]
							.into(),
						ANLOG * 2000000,
					),
				],
				disable_tss,
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
		Some(properties),
		// Extensions
		None,
		wasm_binary,
	))
}

/// Helper to generate genesis storage state.
fn generate_analog_genesis(
	root_key: AccountId,
	council_key: AccountId,
	initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId)>,
	endowed_accounts: Vec<(AccountId, Balance)>,
	disable_tss: bool,
) -> GenesisConfig {
	let initial_nominators: Vec<AccountId> = vec![];
	let locked = PER_VALIDATOR_STASH - PER_VALIDATOR_UNLOCKED;
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.1.clone(), x.0.clone(), locked, StakerStatus::<AccountId>::Validator))
		.chain(initial_nominators.iter().map(|x| {
			let nominations = initial_authorities
				.as_slice()
				.iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), locked, StakerStatus::<AccountId>::Nominator(nominations))
		}))
		.collect::<Vec<_>>();
	let (shard_size, shard_threshold) =
		if disable_tss { (1, 1) } else { (SHARD_SIZE, SHARD_THRESHOLD) };
	GenesisConfig {
		system: SystemConfig { ..Default::default() },
		balances: BalancesConfig {
			// Configure pool accounts with its initial supply.
			balances: endowed_accounts,
		},
		babe: timechain_runtime::BabeConfig {
			authorities: vec![],
			epoch_config: Some(timechain_runtime::BABE_GENESIS_EPOCH_CONFIG),
			..Default::default()
		},
		elections: ElectionsConfig {
			shard_size,
			shard_threshold,
			..Default::default()
		},
		grandpa: GrandpaConfig {
			authorities: vec![],
			..Default::default()
		},
		networks: timechain_runtime::NetworksConfig {
			networks: vec![
				("ethereum".into(), "mainnet".into()),
				("astar".into(), "astar".into()),
				("polygon".into(), "mainnet".into()),
				("ethereum".into(), "dev".into()),
				("ethereum".into(), "goerli".into()),
				("ethereum".into(), "sepolia".into()),
				("astar".into(), "dev".into()),
			],
			..Default::default()
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
		im_online: ImOnlineConfig { keys: vec![] },
		session: timechain_runtime::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.1.clone(),
						timechain_runtime::opaque::SessionKeys {
							babe: x.2.clone(),
							grandpa: x.3.clone(),
							im_online: x.4.clone(),
						},
					)
				})
				.collect::<Vec<_>>(),
		},

		// staking: Default::default(),
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: MIN_VALIDATOR_COUNT,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			// TODO: ForceEra::ForceNone
			..Default::default()
		},
		treasury: Default::default(),
		council: CouncilConfig {
			members: vec![council_key],
			..Default::default()
		},
	}
}

/// Helper to generate genesis storage state.
fn analog_genesis_json(
	root_key: AccountId,
	council_key: AccountId,
	initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId)>,
	endowed_accounts: Vec<(AccountId, Balance)>,
) -> serde_json::Value {
	let initial_nominators: Vec<AccountId> = vec![];
	let locked = PER_VALIDATOR_STASH - PER_VALIDATOR_UNLOCKED;
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.1.clone(), x.0.clone(), locked, StakerStatus::<AccountId>::Validator))
		.chain(initial_nominators.iter().map(|x| {
			let nominations = initial_authorities
				.as_slice()
				.iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), locked, StakerStatus::<AccountId>::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts,
		},
		"babe": {
			"epochConfig": Some(timechain_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		"elections": {
			"shardSize": 12,
			"shardThreshold": 8,
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
			"key": Some(root_key),
		},
		"session": {
			"keys": initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.1.clone(),
						timechain_runtime::opaque::SessionKeys {
							babe: x.2.clone(),
							grandpa: x.3.clone(),
							im_online: x.4.clone(),
						},
					)
				})
				.collect::<Vec<_>>(),
		},

		"staking": {
			"validatorCount": initial_authorities.len() as u32,
			"minimumValidatorCount": MIN_VALIDATOR_COUNT,
			"invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			"slashRewardFraction": Perbill::from_percent(10),
			"stakers": stakers
		},
		"council": {
			"members": vec![council_key],
		},
	})
}
