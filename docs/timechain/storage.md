---
title: Storage
---

The following sections contain Storage methods are part of the default Timechain runtime. On the api, these are exposed via `api.query.<module>.<method>`. 

(NOTE: These were generated from a static/snapshot view of a recent default Timechain runtime. Some items may not be available in older nodes, or in any customized implementations.)

- **[authorship](#authorship)**

- **[babe](#babe)**

- **[balances](#balances)**

- **[council](#council)**

- **[electionProviderMultiPhase](#electionprovidermultiphase)**

- **[grandpa](#grandpa)**

- **[historical](#historical)**

- **[imOnline](#imonline)**

- **[offences](#offences)**

- **[palletProxy](#palletproxy)**

- **[session](#session)**

- **[staking](#staking)**

- **[substrate](#substrate)**

- **[sudo](#sudo)**

- **[system](#system)**

- **[taskMeta](#taskmeta)**

- **[taskSchedule](#taskschedule)**

- **[tesseractSigStorage](#tesseractsigstorage)**

- **[timestamp](#timestamp)**

- **[transactionPayment](#transactionpayment)**

- **[treasury](#treasury)**

- **[vesting](#vesting)**

- **[voterList](#voterlist)**


___


## authorship
 
### author(): `Option<AccountId32>`
- **interface**: `api.query.authorship.author`
- **summary**:    Author of current block. 

___


## babe
 
### authorities(): `Vec<(SpConsensusBabeAppPublic,u64)>`
- **interface**: `api.query.babe.authorities`
- **summary**:    Current epoch authorities. 
 
### authorVrfRandomness(): `Option<[u8;32]>`
- **interface**: `api.query.babe.authorVrfRandomness`
- **summary**:    This field should always be populated during block processing unless  secondary plain slots are enabled (which don't contain a VRF output). 

   It is set in `on_finalize`, before it will contain the value from the last block. 
 
### currentSlot(): `u64`
- **interface**: `api.query.babe.currentSlot`
- **summary**:    Current slot number. 
 
### epochConfig(): `Option<SpConsensusBabeBabeEpochConfiguration>`
- **interface**: `api.query.babe.epochConfig`
- **summary**:    The configuration for the current epoch. Should never be `None` as it is initialized in  genesis. 
 
### epochIndex(): `u64`
- **interface**: `api.query.babe.epochIndex`
- **summary**:    Current epoch index. 
 
### epochStart(): `(u32,u32)`
- **interface**: `api.query.babe.epochStart`
- **summary**:    The block numbers when the last and current epoch have started, respectively `N-1` and  `N`.  NOTE: We track this is in order to annotate the block number when a given pool of  entropy was fixed (i.e. it was known to chain observers). Since epochs are defined in  slots, which may be skipped, the block numbers may not line up with the slot numbers. 
 
### genesisSlot(): `u64`
- **interface**: `api.query.babe.genesisSlot`
- **summary**:    The slot at which the first epoch actually started. This is 0  until the first block of the chain. 
 
### initialized(): `Option<Option<SpConsensusBabeDigestsPreDigest>>`
- **interface**: `api.query.babe.initialized`
- **summary**:    Temporary value (cleared at block finalization) which is `Some`  if per-block initialization has already been called for current block. 
 
### lateness(): `u32`
- **interface**: `api.query.babe.lateness`
- **summary**:    How late the current block is compared to its parent. 

   This entry is populated as part of block execution and is cleaned up  on block finalization. Querying this storage entry outside of block  execution context should always yield zero. 
 
### nextAuthorities(): `Vec<(SpConsensusBabeAppPublic,u64)>`
- **interface**: `api.query.babe.nextAuthorities`
- **summary**:    Next epoch authorities. 
 
### nextEpochConfig(): `Option<SpConsensusBabeBabeEpochConfiguration>`
- **interface**: `api.query.babe.nextEpochConfig`
- **summary**:    The configuration for the next epoch, `None` if the config will not change  (you can fallback to `EpochConfig` instead in that case). 
 
### nextRandomness(): `[u8;32]`
- **interface**: `api.query.babe.nextRandomness`
- **summary**:    Next epoch randomness. 
 
### pendingEpochConfigChange(): `Option<SpConsensusBabeDigestsNextConfigDescriptor>`
- **interface**: `api.query.babe.pendingEpochConfigChange`
- **summary**:    Pending epoch configuration change that will be applied when the next epoch is enacted. 
 
### randomness(): `[u8;32]`
- **interface**: `api.query.babe.randomness`
- **summary**:    The epoch randomness for the *current* epoch. 

   #### Security 

   This MUST NOT be used for gambling, as it can be influenced by a  malicious validator in the short term. It MAY be used in many  cryptographic protocols, however, so long as one remembers that this  (like everything else on-chain) it is public. For example, it can be  used where a number is needed that cannot have been chosen by an  adversary, for purposes such as public-coin zero-knowledge proofs. 
 
### segmentIndex(): `u32`
- **interface**: `api.query.babe.segmentIndex`
- **summary**:    Randomness under construction. 

   We make a trade-off between storage accesses and list length.  We store the under-construction randomness in segments of up to  `UNDER_CONSTRUCTION_SEGMENT_LENGTH`. 

   Once a segment reaches this length, we begin the next one.  We reset all segments and return to `0` at the beginning of every  epoch. 
 
### skippedEpochs(): `Vec<(u64,u32)>`
- **interface**: `api.query.babe.skippedEpochs`
- **summary**:    A list of the last 100 skipped epochs and the corresponding session index  when the epoch was skipped. 

   This is only used for validating equivocation proofs. An equivocation proof  must contains a key-ownership proof for a given session, therefore we need a  way to tie together sessions and epoch indices, i.e. we need to validate that  a validator was the owner of a given key on a given session, and what the  active epoch index was during that session. 
 
### underConstruction(`u32`): `Vec<[u8;32]>`
- **interface**: `api.query.babe.underConstruction`
- **summary**:    TWOX-NOTE: `SegmentIndex` is an increasing integer, so this is okay. 

___


## balances
 
### account(`AccountId32`): `PalletBalancesAccountData`
- **interface**: `api.query.balances.account`
- **summary**:    The Balances pallet example of storing the balance of an account. 

   #### Example 

   ```nocompile  impl pallet_balances::Config for Runtime {  type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>  }  ``` 

   You can also store the balance of an account in the `System` pallet. 

   #### Example 

   ```nocompile  impl pallet_balances::Config for Runtime {  type AccountStore = System  }  ``` 

   But this comes with tradeoffs, storing account balances in the system pallet stores  `frame_system` data alongside the account data contrary to storing account balances in the  `Balances` pallet, which uses a `StorageMap` to store balances data only.  NOTE: This is only used in the case that this pallet is used to store balances. 
 
### freezes(`AccountId32`): `Vec<PalletBalancesIdAmount>`
- **interface**: `api.query.balances.freezes`
- **summary**:    Freeze locks on account balances. 
 
### holds(`AccountId32`): `Vec<PalletBalancesIdAmount>`
- **interface**: `api.query.balances.holds`
- **summary**:    Holds on account balances. 
 
### inactiveIssuance(): `u128`
- **interface**: `api.query.balances.inactiveIssuance`
- **summary**:    The total units of outstanding deactivated balance in the system. 
 
### locks(`AccountId32`): `Vec<PalletBalancesBalanceLock>`
- **interface**: `api.query.balances.locks`
- **summary**:    Any liquidity locks on some account balances.  NOTE: Should only be accessed when setting, changing and freeing a lock. 
 
### reserves(`AccountId32`): `Vec<PalletBalancesReserveData>`
- **interface**: `api.query.balances.reserves`
- **summary**:    Named reserves on some account balances. 
 
### totalIssuance(): `u128`
- **interface**: `api.query.balances.totalIssuance`
- **summary**:    The total units issued in the system. 

___


## council
 
### members(): `Vec<AccountId32>`
- **interface**: `api.query.council.members`
- **summary**:    The current members of the collective. This is stored sorted (just by value). 
 
### prime(): `Option<AccountId32>`
- **interface**: `api.query.council.prime`
- **summary**:    The prime member that helps determine the default vote behavior in case of absentations. 
 
### proposalCount(): `u32`
- **interface**: `api.query.council.proposalCount`
- **summary**:    Proposals so far. 
 
### proposalOf(`H256`): `Option<Call>`
- **interface**: `api.query.council.proposalOf`
- **summary**:    Actual proposal for a given hash, if it's current. 
 
### proposals(): `Vec<H256>`
- **interface**: `api.query.council.proposals`
- **summary**:    The hashes of the active proposals. 
 
### voting(`H256`): `Option<PalletCollectiveVotes>`
- **interface**: `api.query.council.voting`
- **summary**:    Votes on a given proposal, if it is ongoing. 

___


## electionProviderMultiPhase
 
### currentPhase(): `PalletElectionProviderMultiPhasePhase`
- **interface**: `api.query.electionProviderMultiPhase.currentPhase`
- **summary**:    Current phase. 
 
### desiredTargets(): `Option<u32>`
- **interface**: `api.query.electionProviderMultiPhase.desiredTargets`
- **summary**:    Desired number of targets to elect for this round. 

   Only exists when [`Snapshot`] is present. 
 
### minimumUntrustedScore(): `Option<SpNposElectionsElectionScore>`
- **interface**: `api.query.electionProviderMultiPhase.minimumUntrustedScore`
- **summary**:    The minimum score that each 'untrusted' solution must attain in order to be considered  feasible. 

   Can be set via `set_minimum_untrusted_score`. 
 
### queuedSolution(): `Option<PalletElectionProviderMultiPhaseReadySolution>`
- **interface**: `api.query.electionProviderMultiPhase.queuedSolution`
- **summary**:    Current best solution, signed or unsigned, queued to be returned upon `elect`. 

   Always sorted by score. 
 
### round(): `u32`
- **interface**: `api.query.electionProviderMultiPhase.round`
- **summary**:    Internal counter for the number of rounds. 

   This is useful for de-duplication of transactions submitted to the pool, and general  diagnostics of the pallet. 

   This is merely incremented once per every time that an upstream `elect` is called. 
 
### signedSubmissionIndices(): `Vec<(SpNposElectionsElectionScore,u32,u32)>`
- **interface**: `api.query.electionProviderMultiPhase.signedSubmissionIndices`
- **summary**:    A sorted, bounded vector of `(score, block_number, index)`, where each `index` points to a  value in `SignedSubmissions`. 

   We never need to process more than a single signed submission at a time. Signed submissions  can be quite large, so we're willing to pay the cost of multiple database accesses to access  them one at a time instead of reading and decoding all of them at once. 
 
### signedSubmissionNextIndex(): `u32`
- **interface**: `api.query.electionProviderMultiPhase.signedSubmissionNextIndex`
- **summary**:    The next index to be assigned to an incoming signed submission. 

   Every accepted submission is assigned a unique index; that index is bound to that particular  submission for the duration of the election. On election finalization, the next index is  reset to 0. 

   We can't just use `SignedSubmissionIndices.len()`, because that's a bounded set; past its  capacity, it will simply saturate. We can't just iterate over `SignedSubmissionsMap`,  because iteration is slow. Instead, we store the value here. 
 
### signedSubmissionsMap(`u32`): `Option<PalletElectionProviderMultiPhaseSignedSignedSubmission>`
- **interface**: `api.query.electionProviderMultiPhase.signedSubmissionsMap`
- **summary**:    Unchecked, signed solutions. 

   Together with `SubmissionIndices`, this stores a bounded set of `SignedSubmissions` while  allowing us to keep only a single one in memory at a time. 

   Twox note: the key of the map is an auto-incrementing index which users cannot inspect or  affect; we shouldn't need a cryptographically secure hasher. 
 
### snapshot(): `Option<PalletElectionProviderMultiPhaseRoundSnapshot>`
- **interface**: `api.query.electionProviderMultiPhase.snapshot`
- **summary**:    Snapshot data of the round. 

   This is created at the beginning of the signed phase and cleared upon calling `elect`. 
 
### snapshotMetadata(): `Option<PalletElectionProviderMultiPhaseSolutionOrSnapshotSize>`
- **interface**: `api.query.electionProviderMultiPhase.snapshotMetadata`
- **summary**:    The metadata of the [`RoundSnapshot`] 

   Only exists when [`Snapshot`] is present. 

___


## grandpa
 
### currentSetId(): `u64`
- **interface**: `api.query.grandpa.currentSetId`
- **summary**:    The number of changes (both in terms of keys and underlying economic responsibilities)  in the "set" of Grandpa validators from genesis. 
 
### nextForced(): `Option<u32>`
- **interface**: `api.query.grandpa.nextForced`
- **summary**:    next block number where we can force a change. 
 
### pendingChange(): `Option<PalletGrandpaStoredPendingChange>`
- **interface**: `api.query.grandpa.pendingChange`
- **summary**:    Pending change: (signaled at, scheduled change). 
 
### setIdSession(`u64`): `Option<u32>`
- **interface**: `api.query.grandpa.setIdSession`
- **summary**:    A mapping from grandpa set ID to the index of the *most recent* session for which its  members were responsible. 

   This is only used for validating equivocation proofs. An equivocation proof must  contains a key-ownership proof for a given session, therefore we need a way to tie  together sessions and GRANDPA set ids, i.e. we need to validate that a validator  was the owner of a given key on a given session, and what the active set ID was  during that session. 

   TWOX-NOTE: `SetId` is not under user control. 
 
### stalled(): `Option<(u32,u32)>`
- **interface**: `api.query.grandpa.stalled`
- **summary**:    `true` if we are currently stalled. 
 
### state(): `PalletGrandpaStoredState`
- **interface**: `api.query.grandpa.state`
- **summary**:    State of the current authority set. 

___


## historical
 
### historicalSessions(`u32`): `Option<(H256,u32)>`
- **interface**: `api.query.historical.historicalSessions`
- **summary**:    Mapping from historical session indices to session-data root hash and validator count. 
 
### storedRange(): `Option<(u32,u32)>`
- **interface**: `api.query.historical.storedRange`
- **summary**:    The range of historical sessions we store. [first, last) 

___


## imOnline
 
### authoredBlocks(`u32, AccountId32`): `u32`
- **interface**: `api.query.imOnline.authoredBlocks`
- **summary**:    For each session index, we keep a mapping of `ValidatorId<T>` to the  number of blocks authored by the given authority. 
 
### heartbeatAfter(): `u32`
- **interface**: `api.query.imOnline.heartbeatAfter`
- **summary**:    The block number after which it's ok to send heartbeats in the current  session. 

   At the beginning of each session we set this to a value that should fall  roughly in the middle of the session duration. The idea is to first wait for  the validators to produce a block in the current session, so that the  heartbeat later on will not be necessary. 

   This value will only be used as a fallback if we fail to get a proper session  progress estimate from `NextSessionRotation`, as those estimates should be  more accurate then the value we calculate for `HeartbeatAfter`. 
 
### keys(): `Vec<PalletImOnlineSr25519AppSr25519Public>`
- **interface**: `api.query.imOnline.keys`
- **summary**:    The current set of keys that may issue a heartbeat. 
 
### receivedHeartbeats(`u32, u32`): `Option<bool>`
- **interface**: `api.query.imOnline.receivedHeartbeats`
- **summary**:    For each session index, we keep a mapping of `SessionIndex` and `AuthIndex`. 

___


## offences
 
### concurrentReportsIndex(`[u8;16], Bytes`): `Vec<H256>`
- **interface**: `api.query.offences.concurrentReportsIndex`
- **summary**:    A vector of reports of the same kind that happened at the same time slot. 
 
### reports(`H256`): `Option<SpStakingOffenceOffenceDetails>`
- **interface**: `api.query.offences.reports`
- **summary**:    The primary structure that holds all offence records keyed by report identifiers. 

___


## palletProxy
 
### proxyStorage(`AccountId32`): `Option<TimePrimitivesProxyAccStatus>`
- **interface**: `api.query.palletProxy.proxyStorage`

___


## session
 
### currentIndex(): `u32`
- **interface**: `api.query.session.currentIndex`
- **summary**:    Current index of the session. 
 
### disabledValidators(): `Vec<u32>`
- **interface**: `api.query.session.disabledValidators`
- **summary**:    Indices of disabled validators. 

   The vec is always kept sorted so that we can find whether a given validator is  disabled using binary search. It gets cleared when `on_session_ending` returns  a new set of identities. 
 
### keyOwner(`(SpCoreCryptoKeyTypeId,Bytes)`): `Option<AccountId32>`
- **interface**: `api.query.session.keyOwner`
- **summary**:    The owner of a key. The key is the `KeyTypeId` + the encoded key. 
 
### nextKeys(`AccountId32`): `Option<TimechainRuntimeOpaqueSessionKeys>`
- **interface**: `api.query.session.nextKeys`
- **summary**:    The next session keys for a validator. 
 
### queuedChanged(): `bool`
- **interface**: `api.query.session.queuedChanged`
- **summary**:    True if the underlying economic identities or weighting behind the validators  has changed in the queued validator set. 
 
### queuedKeys(): `Vec<(AccountId32,TimechainRuntimeOpaqueSessionKeys)>`
- **interface**: `api.query.session.queuedKeys`
- **summary**:    The queued keys for the next session. When the next session begins, these keys  will be used to determine the validator's session keys. 
 
### validators(): `Vec<AccountId32>`
- **interface**: `api.query.session.validators`
- **summary**:    The current set of validators. 

___


## staking
 
### activeEra(): `Option<PalletStakingActiveEraInfo>`
- **interface**: `api.query.staking.activeEra`
- **summary**:    The active era information, it holds index and start. 

   The active era is the era being currently rewarded. Validator set of this era must be  equal to [`SessionInterface::validators`]. 
 
### bonded(`AccountId32`): `Option<AccountId32>`
- **interface**: `api.query.staking.bonded`
- **summary**:    Map from all locked "stash" accounts to the controller account. 

   TWOX-NOTE: SAFE since `AccountId` is a secure hash. 
 
### bondedEras(): `Vec<(u32,u32)>`
- **interface**: `api.query.staking.bondedEras`
- **summary**:    A mapping from still-bonded eras to the first session index of that era. 

   Must contains information for eras for the range:  `[active_era - bounding_duration; active_era]` 
 
### canceledSlashPayout(): `u128`
- **interface**: `api.query.staking.canceledSlashPayout`
- **summary**:    The amount of currency given to reporters of a slash event which was  canceled by extraordinary circumstances (e.g. governance). 
 
### chillThreshold(): `Option<Percent>`
- **interface**: `api.query.staking.chillThreshold`
- **summary**:    The threshold for when users can start calling `chill_other` for other validators /  nominators. The threshold is compared to the actual number of validators / nominators  (`CountFor*`) in the system compared to the configured max (`Max*Count`). 
 
### counterForNominators(): `u32`
- **interface**: `api.query.staking.counterForNominators`
- **summary**:    Counter for the related counted storage map 
 
### counterForValidators(): `u32`
- **interface**: `api.query.staking.counterForValidators`
- **summary**:    Counter for the related counted storage map 
 
### currentEra(): `Option<u32>`
- **interface**: `api.query.staking.currentEra`
- **summary**:    The current era index. 

   This is the latest planned era, depending on how the Session pallet queues the validator  set, it might be active or not. 
 
### currentPlannedSession(): `u32`
- **interface**: `api.query.staking.currentPlannedSession`
- **summary**:    The last planned session scheduled by the session pallet. 

   This is basically in sync with the call to [`pallet_session::SessionManager::new_session`]. 
 
### erasRewardPoints(`u32`): `PalletStakingEraRewardPoints`
- **interface**: `api.query.staking.erasRewardPoints`
- **summary**:    Rewards for the last `HISTORY_DEPTH` eras.  If reward hasn't been set or has been removed then 0 reward is returned. 
 
### erasStakers(`u32, AccountId32`): `PalletStakingExposure`
- **interface**: `api.query.staking.erasStakers`
- **summary**:    Exposure of validator at era. 

   This is keyed first by the era index to allow bulk deletion and then the stash account. 

   Is it removed after `HISTORY_DEPTH` eras.  If stakers hasn't been set or has been removed then empty exposure is returned. 
 
### erasStakersClipped(`u32, AccountId32`): `PalletStakingExposure`
- **interface**: `api.query.staking.erasStakersClipped`
- **summary**:    Clipped Exposure of validator at era. 

   This is similar to [`ErasStakers`] but number of nominators exposed is reduced to the  `T::MaxNominatorRewardedPerValidator` biggest stakers.  (Note: the field `total` and `own` of the exposure remains unchanged).  This is used to limit the i/o cost for the nominator payout. 

   This is keyed fist by the era index to allow bulk deletion and then the stash account. 

   Is it removed after `HISTORY_DEPTH` eras.  If stakers hasn't been set or has been removed then empty exposure is returned. 
 
### erasStartSessionIndex(`u32`): `Option<u32>`
- **interface**: `api.query.staking.erasStartSessionIndex`
- **summary**:    The session index at which the era start for the last `HISTORY_DEPTH` eras. 

   Note: This tracks the starting session (i.e. session index when era start being active)  for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`. 
 
### erasTotalStake(`u32`): `u128`
- **interface**: `api.query.staking.erasTotalStake`
- **summary**:    The total amount staked for the last `HISTORY_DEPTH` eras.  If total hasn't been set or has been removed then 0 stake is returned. 
 
### erasValidatorPrefs(`u32, AccountId32`): `PalletStakingValidatorPrefs`
- **interface**: `api.query.staking.erasValidatorPrefs`
- **summary**:    Similar to `ErasStakers`, this holds the preferences of validators. 

   This is keyed first by the era index to allow bulk deletion and then the stash account. 

   Is it removed after `HISTORY_DEPTH` eras. 
 
### erasValidatorReward(`u32`): `Option<u128>`
- **interface**: `api.query.staking.erasValidatorReward`
- **summary**:    The total validator era payout for the last `HISTORY_DEPTH` eras. 

   Eras that haven't finished yet or has been removed doesn't have reward. 
 
### forceEra(): `PalletStakingForcing`
- **interface**: `api.query.staking.forceEra`
- **summary**:    Mode of era forcing. 
 
### invulnerables(): `Vec<AccountId32>`
- **interface**: `api.query.staking.invulnerables`
- **summary**:    Any validators that may never be slashed or forcibly kicked. It's a Vec since they're  easy to initialize and the performance hit is minimal (we expect no more than four  invulnerables) and restricted to testnets. 
 
### ledger(`AccountId32`): `Option<PalletStakingStakingLedger>`
- **interface**: `api.query.staking.ledger`
- **summary**:    Map from all (unlocked) "controller" accounts to the info regarding the staking. 
 
### maxNominatorsCount(): `Option<u32>`
- **interface**: `api.query.staking.maxNominatorsCount`
- **summary**:    The maximum nominator count before we stop allowing new validators to join. 

   When this value is not set, no limits are enforced. 
 
### maxValidatorsCount(): `Option<u32>`
- **interface**: `api.query.staking.maxValidatorsCount`
- **summary**:    The maximum validator count before we stop allowing new validators to join. 

   When this value is not set, no limits are enforced. 
 
### minCommission(): `Perbill`
- **interface**: `api.query.staking.minCommission`
- **summary**:    The minimum amount of commission that validators can set. 

   If set to `0`, no limit exists. 
 
### minimumActiveStake(): `u128`
- **interface**: `api.query.staking.minimumActiveStake`
- **summary**:    The minimum active nominator stake of the last successful election. 
 
### minimumValidatorCount(): `u32`
- **interface**: `api.query.staking.minimumValidatorCount`
- **summary**:    Minimum number of staking participants before emergency conditions are imposed. 
 
### minNominatorBond(): `u128`
- **interface**: `api.query.staking.minNominatorBond`
- **summary**:    The minimum active bond to become and maintain the role of a nominator. 
 
### minValidatorBond(): `u128`
- **interface**: `api.query.staking.minValidatorBond`
- **summary**:    The minimum active bond to become and maintain the role of a validator. 
 
### nominators(`AccountId32`): `Option<PalletStakingNominations>`
- **interface**: `api.query.staking.nominators`
- **summary**:    The map from nominator stash key to their nomination preferences, namely the validators that  they wish to support. 

   Note that the keys of this storage map might become non-decodable in case the  [`Config::MaxNominations`] configuration is decreased. In this rare case, these nominators  are still existent in storage, their key is correct and retrievable (i.e. `contains_key`  indicates that they exist), but their value cannot be decoded. Therefore, the non-decodable  nominators will effectively not-exist, until they re-submit their preferences such that it  is within the bounds of the newly set `Config::MaxNominations`. 

   This implies that `::iter_keys().count()` and `::iter().count()` might return different  values for this map. Moreover, the main `::count()` is aligned with the former, namely the  number of keys that exist. 

   Lastly, if any of the nominators become non-decodable, they can be chilled immediately via  [`Call::chill_other`] dispatchable by anyone. 

   TWOX-NOTE: SAFE since `AccountId` is a secure hash. 
 
### nominatorSlashInEra(`u32, AccountId32`): `Option<u128>`
- **interface**: `api.query.staking.nominatorSlashInEra`
- **summary**:    All slashing events on nominators, mapped by era to the highest slash value of the era. 
 
### offendingValidators(): `Vec<(u32,bool)>`
- **interface**: `api.query.staking.offendingValidators`
- **summary**:    Indices of validators that have offended in the active era and whether they are currently  disabled. 

   This value should be a superset of disabled validators since not all offences lead to the  validator being disabled (if there was no slash). This is needed to track the percentage of  validators that have offended in the current era, ensuring a new era is forced if  `OffendingValidatorsThreshold` is reached. The vec is always kept sorted so that we can find  whether a given validator has previously offended using binary search. It gets cleared when  the era ends. 
 
### payee(`AccountId32`): `PalletStakingRewardDestination`
- **interface**: `api.query.staking.payee`
- **summary**:    Where the reward payment should be made. Keyed by stash. 

   TWOX-NOTE: SAFE since `AccountId` is a secure hash. 
 
### slashingSpans(`AccountId32`): `Option<PalletStakingSlashingSlashingSpans>`
- **interface**: `api.query.staking.slashingSpans`
- **summary**:    Slashing spans for stash accounts. 
 
### slashRewardFraction(): `Perbill`
- **interface**: `api.query.staking.slashRewardFraction`
- **summary**:    The percentage of the slash that is distributed to reporters. 

   The rest of the slashed value is handled by the `Slash`. 
 
### spanSlash(`(AccountId32,u32)`): `PalletStakingSlashingSpanRecord`
- **interface**: `api.query.staking.spanSlash`
- **summary**:    Records information about the maximum slash of a stash within a slashing span,  as well as how much reward has been paid out. 
 
### unappliedSlashes(`u32`): `Vec<PalletStakingUnappliedSlash>`
- **interface**: `api.query.staking.unappliedSlashes`
- **summary**:    All unapplied slashes that are queued for later. 
 
### validatorCount(): `u32`
- **interface**: `api.query.staking.validatorCount`
- **summary**:    The ideal number of active validators. 
 
### validators(`AccountId32`): `PalletStakingValidatorPrefs`
- **interface**: `api.query.staking.validators`
- **summary**:    The map from (wannabe) validator stash key to the preferences of that validator. 

   TWOX-NOTE: SAFE since `AccountId` is a secure hash. 
 
### validatorSlashInEra(`u32, AccountId32`): `Option<(Perbill,u128)>`
- **interface**: `api.query.staking.validatorSlashInEra`
- **summary**:    All slashing events on validators, mapped by era to the highest slash proportion  and slash value of the era. 

___


## substrate

_These are well-known keys that are always available to the runtime implementation of any Substrate-based network._
 
### changesTrieConfig(): `u32`
- **interface**: `api.query.substrate.changesTrieConfig`
- **summary**:    Changes trie configuration is stored under this key. 
 
### childStorageKeyPrefix(): `u32`
- **interface**: `api.query.substrate.childStorageKeyPrefix`
- **summary**:    Prefix of child storage keys. 
 
### code(): `Bytes`
- **interface**: `api.query.substrate.code`
- **summary**:    Wasm code of the runtime. 
 
### extrinsicIndex(): `u32`
- **interface**: `api.query.substrate.extrinsicIndex`
- **summary**:    Current extrinsic index (u32) is stored under this key. 
 
### heapPages(): `u64`
- **interface**: `api.query.substrate.heapPages`
- **summary**:    Number of wasm linear memory pages required for execution of the runtime. 

___


## sudo
 
### key(): `Option<AccountId32>`
- **interface**: `api.query.sudo.key`
- **summary**:    The `AccountId` of the sudo key. 

___


## system
 
### account(`AccountId32`): `FrameSystemAccountInfo`
- **interface**: `api.query.system.account`
- **summary**:    The full account information for a particular account ID. 
 
### allExtrinsicsLen(): `Option<u32>`
- **interface**: `api.query.system.allExtrinsicsLen`
- **summary**:    Total length (in bytes) for all extrinsics put together, for the current block. 
 
### blockHash(`u32`): `H256`
- **interface**: `api.query.system.blockHash`
- **summary**:    Map of block numbers to block hashes. 
 
### blockWeight(): `FrameSupportDispatchPerDispatchClassWeight`
- **interface**: `api.query.system.blockWeight`
- **summary**:    The current weight for the block. 
 
### digest(): `SpRuntimeDigest`
- **interface**: `api.query.system.digest`
- **summary**:    Digest of the current block, also part of the block header. 
 
### eventCount(): `u32`
- **interface**: `api.query.system.eventCount`
- **summary**:    The number of events in the `Events<T>` list. 
 
### events(): `Vec<FrameSystemEventRecord>`
- **interface**: `api.query.system.events`
- **summary**:    Events deposited for the current block. 

   NOTE: The item is unbound and should therefore never be read on chain.  It could otherwise inflate the PoV size of a block. 

   Events have a large in-memory size. Box the events to not go out-of-memory  just in case someone still reads them from within the runtime. 
 
### eventTopics(`H256`): `Vec<(u32,u32)>`
- **interface**: `api.query.system.eventTopics`
- **summary**:    Mapping between a topic (represented by T::Hash) and a vector of indexes  of events in the `<Events<T>>` list. 

   All topic vectors have deterministic storage locations depending on the topic. This  allows light-clients to leverage the changes trie storage tracking mechanism and  in case of changes fetch the list of events of interest. 

   The value has the type `(T::BlockNumber, EventIndex)` because if we used only just  the `EventIndex` then in case if the topic has the same contents on the next block  no notification will be triggered thus the event might be lost. 
 
### executionPhase(): `Option<FrameSystemPhase>`
- **interface**: `api.query.system.executionPhase`
- **summary**:    The execution phase of the block. 
 
### extrinsicCount(): `Option<u32>`
- **interface**: `api.query.system.extrinsicCount`
- **summary**:    Total extrinsics count for the current block. 
 
### extrinsicData(`u32`): `Bytes`
- **interface**: `api.query.system.extrinsicData`
- **summary**:    Extrinsics data for the current block (maps an extrinsic's index to its data). 
 
### lastRuntimeUpgrade(): `Option<FrameSystemLastRuntimeUpgradeInfo>`
- **interface**: `api.query.system.lastRuntimeUpgrade`
- **summary**:    Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened. 
 
### number(): `u32`
- **interface**: `api.query.system.number`
- **summary**:    The current block number being processed. Set by `execute_block`. 
 
### parentHash(): `H256`
- **interface**: `api.query.system.parentHash`
- **summary**:    Hash of the previous block. 
 
### upgradedToTripleRefCount(): `bool`
- **interface**: `api.query.system.upgradedToTripleRefCount`
- **summary**:    True if we have upgraded so that AccountInfo contains three types of `RefCount`. False  (default) if not. 
 
### upgradedToU32RefCount(): `bool`
- **interface**: `api.query.system.upgradedToU32RefCount`
- **summary**:    True if we have upgraded so that `type RefCount` is `u32`. False (default) if not. 

___


## taskMeta
 
### collectionMeta(`Text`): `Option<TimePrimitivesAbstractionCollection>`
- **interface**: `api.query.taskMeta.collectionMeta`
 
### payableTaskMetaStorage(`u64`): `Option<TimePrimitivesAbstractionPayableTask>`
- **interface**: `api.query.taskMeta.payableTaskMetaStorage`
 
### taskMetaStorage(`u64`): `Option<TimePrimitivesAbstractionTask>`
- **interface**: `api.query.taskMeta.taskMetaStorage`

___


## taskSchedule
 
### indexerScore(`AccountId32`): `u32`
- **interface**: `api.query.taskSchedule.indexerScore`
 
### lastKey(): `Option<u64>`
- **interface**: `api.query.taskSchedule.lastKey`
 
### payableScheduleStorage(`u64`): `Option<TimePrimitivesAbstractionPayableTaskSchedule>`
- **interface**: `api.query.taskSchedule.payableScheduleStorage`
 
### scheduleStorage(`u64`): `Option<TimePrimitivesAbstractionTaskSchedule>`
- **interface**: `api.query.taskSchedule.scheduleStorage`
 
### shardTasks(`u64`): `Vec<u64>`
- **interface**: `api.query.taskSchedule.shardTasks`
 
### timedOutTasks(): `Vec<u64>`
- **interface**: `api.query.taskSchedule.timedOutTasks`

___


## tesseractSigStorage
 
### chronicleOwner(`AccountId32`): `Option<AccountId32>`
- **interface**: `api.query.tesseractSigStorage.chronicleOwner`
- **summary**:    record the chronicle worker's owner or its validator account id 
 
### commitedOffences(`AccountId32`): `Option<(u8,BTreeSet<AccountId32>)>`
- **interface**: `api.query.tesseractSigStorage.commitedOffences`
 
### getShardsIndex(): `u64`
- **interface**: `api.query.tesseractSigStorage.getShardsIndex`
- **summary**:    Counter for getting (N) next available shard(s)s 
 
### lastCommittedChronicle(`AccountId32`): `u32`
- **interface**: `api.query.tesseractSigStorage.lastCommittedChronicle`
- **summary**:    record the last block number of each chronicle worker commit valid signature 
 
### lastCommittedShard(`u64`): `u32`
- **interface**: `api.query.tesseractSigStorage.lastCommittedShard`
- **summary**:    record the last block number of each shard commit valid signature 
 
### reportedOffences(`AccountId32`): `Option<(u8,BTreeSet<AccountId32>)>`
- **interface**: `api.query.tesseractSigStorage.reportedOffences`
 
### shardId(): `u64`
- **interface**: `api.query.tesseractSigStorage.shardId`
- **summary**:    Counter for creating unique shard_ids during on-chain creation 
 
### signatureStoreData(`u64, u64`): `Option<[u8;64]>`
- **interface**: `api.query.tesseractSigStorage.signatureStoreData`
 
### tssGroupKey(`u64`): `Option<[u8;33]>`
- **interface**: `api.query.tesseractSigStorage.tssGroupKey`
 
### tssShards(`u64`): `Option<PalletTesseractSigStorageShardShardState>`
- **interface**: `api.query.tesseractSigStorage.tssShards`
- **summary**:    Indicates precise members of each TSS set by it's u64 id  Required for key generation and identification 
 
### validatorToChronicle(`AccountId32`): `Option<Vec<AccountId32>>`
- **interface**: `api.query.tesseractSigStorage.validatorToChronicle`
- **summary**:    record the chronicle worker ids for each validator 

___


## timestamp
 
### didUpdate(): `bool`
- **interface**: `api.query.timestamp.didUpdate`
- **summary**:    Did the timestamp get updated in this block? 
 
### now(): `u64`
- **interface**: `api.query.timestamp.now`
- **summary**:    Current time for the current block. 

___


## transactionPayment
 
### nextFeeMultiplier(): `u128`
- **interface**: `api.query.transactionPayment.nextFeeMultiplier`
 
### storageVersion(): `PalletTransactionPaymentReleases`
- **interface**: `api.query.transactionPayment.storageVersion`

___


## treasury
 
### approvals(): `Vec<u32>`
- **interface**: `api.query.treasury.approvals`
- **summary**:    Proposal indices that have been approved but not yet awarded. 
 
### deactivated(): `u128`
- **interface**: `api.query.treasury.deactivated`
- **summary**:    The amount which has been reported as inactive to Currency. 
 
### proposalCount(): `u32`
- **interface**: `api.query.treasury.proposalCount`
- **summary**:    Number of proposals that have been made. 
 
### proposals(`u32`): `Option<PalletTreasuryProposal>`
- **interface**: `api.query.treasury.proposals`
- **summary**:    Proposals that have been made. 

___


## vesting
 
### vestingSchedules(`AccountId32`): `Vec<AnalogVestingVestingSchedule>`
- **interface**: `api.query.vesting.vestingSchedules`
- **summary**:    Vesting schedules of an account. 

   VestingSchedules: map AccountId => Vec<VestingSchedule> 

___


## voterList
 
### counterForListNodes(): `u32`
- **interface**: `api.query.voterList.counterForListNodes`
- **summary**:    Counter for the related counted storage map 
 
### listBags(`u64`): `Option<PalletBagsListListBag>`
- **interface**: `api.query.voterList.listBags`
- **summary**:    A bag stored in storage. 

   Stores a `Bag` struct, which stores head and tail pointers to itself. 
 
### listNodes(`AccountId32`): `Option<PalletBagsListListNode>`
- **interface**: `api.query.voterList.listNodes`
- **summary**:    A single node, within some bag. 

   Nodes store links forward and back within their respective bags. 
