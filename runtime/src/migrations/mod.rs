use polkadot_sdk::*;

use sp_core::crypto::Ss58Codec;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as DiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;

use pallet_staking::{ConfigOp, RewardDestination, ValidatorPrefs};
use sp_staking::StakingInterface;

use frame_support::weights::Weight;

use time_primitives::{AccountId, ANLOG};

use crate::{RuntimeOrigin, Session, SessionKeys, Staking};

const NOMINATOR: AccountId = AccountId::new(sp_core::hex2array!(
	"6d6f646c74696d656e6d706c0001000000000000000000000000000000000000"
));

const AUTHORITIES: [&str; 16] = [
	"an94D6zDW4DKungVMGCg7DMRfWR8Ndm9D6DfzHfZ87DQq9xGX",
	"an6aVKfjeXWSPv1cxZ1BFUb8wANehLnvoaCaVbSFRXQ3eWqCL",
	"anACbX4gaF36jALe8J9GpcMsNgWXEjWsFWdAQU2eLbPjGQm5p",
	"an9ygGKaqRxd5u6ncx8m2DjMYGCqiz6wYGkEpedzTonU1DSCZ",
	"anAFAooFocnHp7fw7qBPiVSuyEvvoK8B4KG8GCUj95ctXSWs1",
	"anAjbDZ6ccpiGnc5FJ6WfFraxpV8dWxTiXoK1J4fATBZd7Nni",
	"an739znrFkGsHecf6XLAF6AkhZPLEMv4pfhXkMaN8mgt5R1m2",
	"an831DuKpiBsWCvwMt7kWNmDiYi6gBpunkrD7TxkCtzaLpeSW",
	"an7Wy9yy11iywTJCQ2vGgQRPez7ZpLp5XsuWCaMmG4JNoXzyF",
	"anA7NdKgb4zLKyGDFypvQfjXwGLxQZwQkH9WETDmXdnziju6o",
	"an5mNU1Um4pgw7XsnnbYvEae2j3roy8WzEW95HKf22YXcCCgv",
	"an82zhY6hhEd6Dp9n3beuvru2QhdAHrF5aJuneni8yJBpeB5H",
	"an62RTVxLyuWGhRvmbgHLCCbXSu2GU5ehYR2ZWB1wavENwWGg",
	"an6zfy8znPMtTXkGTq1JNhyKT49w3g3RoLc7PGP5iJ17n4gzd",
	"an8Wkh4dHqBHt1A9kt1fR2qHhBkSgNsAUNz4SiWvhp3Smk2CZ",
	"an5x5sJAuQtV9CiDWY95zbSJEkaYDk3ozGvJyAJ5oCZuiwRYm",
];

const SESSION_KEYS: [(&str, &str, &str, &str); 16] = [
	(
		"an6MvHDNt1YvRVjn8ysBFe35JHUdYhNJL5Hn8tjjMGXa6X3t7",
		"an8T1DaUBTPVMN5VZPLAGDR52AM589ddDMpdqqNUwQnPRQq6j",
		"anB1sLxfcxRqfp1zoNoBAieE5HYTnyHMX9yxcTmkaY5qu4cru",
		"an9LSg2NhPN7sAHkzjzX9G64FGn2oh8GiVFWQjRh6xQ8GBJWK",
	),
	(
		"an9suWT2b6Hy4kmo3aqJk8xLgjM6zoeqs32mK2wk9ySKKnUhi",
		"an9tbPDPLtQbpRHT7EhLcoqQPmrgYsFzCQJv2zpqSz3UkuTft",
		"an9fTnDaVEAfWBiRHRpB76kKT72SXpZnxmcruvWKjWRVpfySA",
		"an8PqnYJ7eNwDRWjupFnBtLfPsyfPwocBmhwa4Bype3Rc78Dg",
	),
	(
		"anAkBa5ad3CjAUrscDWVhx9vasX2nDKVNCfewUsQw6g3naDy8",
		"an73dMhvPLjW9txz8WZ7BMUdi7hLRhehdvFNH1fXse8uSdmVa",
		"an8qhbbKgSZHNXwB6Fr2y44Yj41F8DyEvis6kUejF9VvtohJq",
		"an8kNMeqxwEaXaALTq7DbRoGbr1v7YJScCzvmPeW2dkwis2W6",
	),
	(
		"anB66MfS6A3WrzKqhBXz8QRp3Y2SvnDz3iH2845gx1RwjmYNo",
		"an6Mg4mCgAGZ1disz2TCLpbJQuGePtGtSMHLAapsmmpn27pUt",
		"an6S9VCVa19AvxUdTKemC9KKkouPpv9nb7Wfr5u1XZw9Cwb7S",
		"an74vg8YzqQjZ5F9KHdV2Ds8TZpyWENEfgDitHys9vcrEifDB",
	),
	(
		"an9kacM6eUc9oUmGUzBdEh6uurLb2oKN1zegXtpfC5kHJ1BNx",
		"an8KRvJqU9Z9SWFCwry1ksSeq2GyKS4qekz7ewvwqeRqeMzJ9",
		"an6b5GnEtknqviybA4hRcjBricCmvLHa9XufEM8RkJPiF3VJP",
		"an7AVdU5fQRoPoNJwuSAGhUfCVsyR3ZSRMmuB2UqqxTjZUeaA",
	),
	(
		"an6hn8dRfxTxyJpa1jzxN3ad2cehGbrYbKuVoBgxrvFzewpHm",
		"an74y21PvUsKFf3Bwoqsc7XmFte8jasHSZ3rBgWtxYzDp5P5A",
		"an8F7ZDmsdRZcktG9UhPqJn3axJnYWLUNfj1TAjVZY6dHCNLT",
		"anAFTM4uofqrVasjbE8ZNVxQgTk6XTjrkEH5higdxMM4XsZh8",
	),
	(
		"an9yhLYc6GFjzk9GK18X3LqriVN9kPSWtXCBveAREHzfVSjVi",
		"anAsPu1Gyft5Q9GDJdBdUjx6KWRqP7m8DW7874nj9LCvSu4oD",
		"anAqAy7EeksF6S2GFQvTFes9pnuRTtdGpUzqbPEs144icdH95",
		"an6qoQEjb6Y7GfXNUwxV4LUC715duy2BrCt7jfhCnMsR26BoN",
	),
	(
		"anAWqSzirRR3LiiXAe6DcniQwHGrCtjqupBWuHis5FnRVra6G",
		"an9qKQs64LH5oVS8Xc27KH47agzKKpb2KxzHaPzHnfH4uAQAg",
		"an6DDiY7338XAW42QNN4UF7BS31sxErpiXwCP5MexMkpWNBre",
		"an9A6Bn5pRFnZHd9C1P8ZpLRqiP53MfyhfUX2mHiAUGBW2TZU",
	),
	(
		"an9X1sMpNEAgd7pz9NDYGFm1u6cD7yEfSqemyFRZow3DdcvkK",
		"an6yJrGFn1KhVZmHsXjVTfVdQg98jS8qQ9KVKPcRBrunjtAhd",
		"an7g6MhuNH9oCcqsPDaz96UDxA1jMqqErUmTc1LqYD5uHPiye",
		"an6ip59GVrUAf1CQWkveiEH5JuDeaDvZAtAWKgEqkUuCscUGY",
	),
	(
		"an84vkVTxzRAq8mLc9JMhXXXkZudLJeNcFe6Cb5DF7n6aKCE3",
		"anAu4XwtBFBous7W2q36XEKFqMrh6iyFyM5K5FiXB9ALnvEJJ",
		"an9ZywA694MLbtTRG9sB3TmrnmQ6pfP1WU7ZNuUSd8h4D2A36",
		"anAvSpwUTX295rvU2QVz2TNnV2h58VhM3cHut2xCgQLx8Gptf",
	),
	(
		"an94P6YjTbjFct8bKAzxZSN4TbKTS4Sf7ehCML6A5QFDbD1P3",
		"anAHavqV15L9K8JYpAdqTKvEnacvewsronVz8WkreRwHmkRqx",
		"anBHkZdgE52hFGmdAHq64rRWhm5ncs5paYFGEhrfdJZKjx55G",
		"an6SQMrxfkrmbfQm8Lya6UynnRZ7iF2xBqzvm9MfKZTPavNer",
	),
	(
		"an6tPMdGyjZEDNVz8thcdzNuHjLVC3TZEhnheNqZ3TebDDo5L",
		"an9fCGteEMqzhTi7h7dvG2Djj47hq9ZoKV2jk6Pb5qyzb7fGF",
		"an8jyZFRma1FGAhSxntJshjLe88w79gFq2Z1nzTNczfmAJH2E",
		"an5xCU4o9XoytuLTk4hhwgJyk5oYhRR7u9rQEqKDoYPe8okTo",
	),
	(
		"an6mMjZY8pvuw4XXbGQzeKEZa7MfCLbh5Fjd9QJZ5NxuLR6pm",
		"an7pYHHqjtMS6qWtMkSLDAAMV1aaUBbtpvkJENk3GicM1EuRv",
		"anBKZma8P54tQdUV8kV4hhS4AYnT3tirdPJFMvju6RP7WfEaz",
		"anABykNvyePvBj6Tdemn1Gx9t5swPwJRtY7GFqn6QRZKWRU7n",
	),
	(
		"an6tzJTWZJKJ9VgffPremaDxJH3Gm1bJjc5MshsFxL13YoDL7",
		"an9CtdY46PJYHWvEM2fH8P4FdDsU1fKAbAxwdWp14TN4sz311",
		"anAeHAVaJnyRKNG6MUnbJ8HLHvg9yJynmyRaHhxZQfGEqhqAT",
		"anACsq8grsz1tv8a8eibKijgKeqjuSj9ByzvqQuR1pCui2ku2",
	),
	(
		"an6pGQmu7AyPDNLRZjXyHZVH5yig1wWSUZ58ppzhQpZMvcyE5",
		"an6yHB82PBtw6M12jsuTBZPTwxMpTJkBW9cEi2KuqJtLsJuU3",
		"an6oFGttbqYsVLjvq9vv5YHta4c84syCzdrSJNxoqJ7PS3n87",
		"an75V9N24cwPs7WgVCTCokn3p7rpDsqZck4gTKzYnXmHxWsaF",
	),
	(
		"an7XwfH6geK6xxP9DaAyBnaTjkmW2DTK9eYRA3K5qReQffg1P",
		"an7vdmEReyyEZWTgvHmKakPbgD2cJqS2zNkPtLUXU1nZiraMz",
		"an6fsWmbCJ1iDdC6WdtvdsbR4HUE17Fhvk1jtsu1acYekEBog",
		"an9JBf52bwKtCiFyGfHTh67n1gMqYMmLYdhiacTjjfSbALdEJ",
	),
];

pub struct ExtendValidatorSet<T>(sp_std::marker::PhantomData<T>);

impl<T> frame_support::traits::OnRuntimeUpgrade for ExtendValidatorSet<T>
where
	T: frame_system::Config + pallet_session::Config + pallet_staking::Config,
	T::AccountId: From<AccountId>,
{
	fn on_runtime_upgrade() -> Weight {
		if Staking::desired_validator_count() != 10 {
			log::warn!("🥩 Migration skipped!");
			return Weight::zero();
		}

		// Wrap storage migration indside transactions that reverts on failure
		if let Err(error) = frame_support::storage::with_storage_layer(|| {
			// Extend validator target and accounts
			Staking::set_validator_count(RuntimeOrigin::root(), 26)?;

			Staking::set_staking_configs(
				RuntimeOrigin::root(),
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Set(26),
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
			)?;

			// Retrieve current nomination
			let mut targets =
				Staking::nominations(&NOMINATOR).ok_or("Failed to retrieve nominations")?;

			for (addr, keys) in sp_std::iter::zip(AUTHORITIES, SESSION_KEYS) {
				let account =
					AccountId::from_ss58check(addr).or(Err("Failed to parse stash address"))?;
				let origin = RuntimeOrigin::signed(account.clone());

				// Bond and enable validation
				Staking::bond(origin.clone(), 100_000 * ANLOG, RewardDestination::Staked)?;
				Staking::validate(origin.clone(), ValidatorPrefs::default())?;

				// Set included session keys
				Session::set_keys(
					origin,
					SessionKeys {
						babe: BabeId::from_ss58check(keys.0).or(Err("Failed to parse babe key"))?,
						grandpa: GrandpaId::from_ss58check(keys.1)
							.or(Err("Failed to parse grandpa key"))?,
						im_online: ImOnlineId::from_ss58check(keys.2)
							.or(Err("Failed to parse heartbeat key"))?,
						authority_discovery: DiscoveryId::from_ss58check(keys.3)
							.or(Err("Failed to parse discovery key"))?,
					},
					sp_std::vec![],
				)?;

				// Collect all new validators
				targets.push(account);
			}

			// Update nominations
			let targets: sp_std::vec::Vec<_> = targets.into_iter().map(Into::into).collect();
			Staking::nominate(RuntimeOrigin::signed(NOMINATOR), targets.clone())?;

			log::info!("🥩 Migration to {} nodes successful.", targets.len());

			Ok::<(), &str>(())
		}) {
			log::error!("🥩 Migration failed: {error}");
		}

		Weight::zero()
	}
}
