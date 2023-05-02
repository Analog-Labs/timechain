---
title: Events
---

Events are emitted for certain operations on the runtime. The following sections describe the events that are part of the default Timechain runtime. 

(NOTE: These were generated from a static/snapshot view of a recent default Timechain runtime. Some items may not be available in older nodes, or in any customized implementations.)

- **[balances](#balances)**

- **[council](#council)**

- **[electionProviderMultiPhase](#electionprovidermultiphase)**

- **[grandpa](#grandpa)**

- **[imOnline](#imonline)**

- **[offences](#offences)**

- **[palletProxy](#palletproxy)**

- **[session](#session)**

- **[staking](#staking)**

- **[sudo](#sudo)**

- **[system](#system)**

- **[taskMeta](#taskmeta)**

- **[taskSchedule](#taskschedule)**

- **[tesseractSigStorage](#tesseractsigstorage)**

- **[transactionPayment](#transactionpayment)**

- **[treasury](#treasury)**

- **[utility](#utility)**

- **[vesting](#vesting)**

- **[voterList](#voterlist)**


___


## balances
 
### BalanceSet(`AccountId32`, `u128`)
- **interface**: `api.events.balances.BalanceSet.is`
- **summary**:    A balance was set by root. 
 
### Burned(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Burned.is`
- **summary**:    Some amount was burned from an account. 
 
### Deposit(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Deposit.is`
- **summary**:    Some amount was deposited (e.g. for transaction fees). 
 
### DustLost(`AccountId32`, `u128`)
- **interface**: `api.events.balances.DustLost.is`
- **summary**:    An account was removed whose balance was non-zero but below ExistentialDeposit,  resulting in an outright loss. 
 
### Endowed(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Endowed.is`
- **summary**:    An account was created with some free balance. 
 
### Frozen(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Frozen.is`
- **summary**:    Some balance was frozen. 
 
### Issued(`u128`)
- **interface**: `api.events.balances.Issued.is`
- **summary**:    Total issuance was increased by `amount`, creating a credit to be balanced. 
 
### Locked(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Locked.is`
- **summary**:    Some balance was locked. 
 
### Minted(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Minted.is`
- **summary**:    Some amount was minted into an account. 
 
### Rescinded(`u128`)
- **interface**: `api.events.balances.Rescinded.is`
- **summary**:    Total issuance was decreased by `amount`, creating a debt to be balanced. 
 
### Reserved(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Reserved.is`
- **summary**:    Some balance was reserved (moved from free to reserved). 
 
### ReserveRepatriated(`AccountId32`, `AccountId32`, `u128`, `FrameSupportTokensMiscBalanceStatus`)
- **interface**: `api.events.balances.ReserveRepatriated.is`
- **summary**:    Some balance was moved from the reserve of the first account to the second account.  Final argument indicates the destination balance type. 
 
### Restored(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Restored.is`
- **summary**:    Some amount was restored into an account. 
 
### Slashed(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Slashed.is`
- **summary**:    Some amount was removed from the account (e.g. for misbehavior). 
 
### Suspended(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Suspended.is`
- **summary**:    Some amount was suspended from an account (it can be restored later). 
 
### Thawed(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Thawed.is`
- **summary**:    Some balance was thawed. 
 
### Transfer(`AccountId32`, `AccountId32`, `u128`)
- **interface**: `api.events.balances.Transfer.is`
- **summary**:    Transfer succeeded. 
 
### Unlocked(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Unlocked.is`
- **summary**:    Some balance was unlocked. 
 
### Unreserved(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Unreserved.is`
- **summary**:    Some balance was unreserved (moved from reserved to free). 
 
### Upgraded(`AccountId32`)
- **interface**: `api.events.balances.Upgraded.is`
- **summary**:    An account was upgraded. 
 
### Withdraw(`AccountId32`, `u128`)
- **interface**: `api.events.balances.Withdraw.is`
- **summary**:    Some amount was withdrawn from the account (e.g. for transaction fees). 

___


## council
 
### Approved(`H256`)
- **interface**: `api.events.council.Approved.is`
- **summary**:    A motion was approved by the required threshold. 
 
### Closed(`H256`, `u32`, `u32`)
- **interface**: `api.events.council.Closed.is`
- **summary**:    A proposal was closed because its threshold was reached or after its duration was up. 
 
### Disapproved(`H256`)
- **interface**: `api.events.council.Disapproved.is`
- **summary**:    A motion was not approved by the required threshold. 
 
### Executed(`H256`, `Result<Null, SpRuntimeDispatchError>`)
- **interface**: `api.events.council.Executed.is`
- **summary**:    A motion was executed; result will be `Ok` if it returned without error. 
 
### MemberExecuted(`H256`, `Result<Null, SpRuntimeDispatchError>`)
- **interface**: `api.events.council.MemberExecuted.is`
- **summary**:    A single member did some action; result will be `Ok` if it returned without error. 
 
### Proposed(`AccountId32`, `u32`, `H256`, `u32`)
- **interface**: `api.events.council.Proposed.is`
- **summary**:    A motion (given hash) has been proposed (by given account) with a threshold (given  `MemberCount`). 
 
### Voted(`AccountId32`, `H256`, `bool`, `u32`, `u32`)
- **interface**: `api.events.council.Voted.is`
- **summary**:    A motion (given hash) has been voted on by given account, leaving  a tally (yes votes and no votes given respectively as `MemberCount`). 

___


## electionProviderMultiPhase
 
### ElectionFailed()
- **interface**: `api.events.electionProviderMultiPhase.ElectionFailed.is`
- **summary**:    An election failed. 

   Not much can be said about which computes failed in the process. 
 
### ElectionFinalized(`PalletElectionProviderMultiPhaseElectionCompute`, `SpNposElectionsElectionScore`)
- **interface**: `api.events.electionProviderMultiPhase.ElectionFinalized.is`
- **summary**:    The election has been finalized, with the given computation and score. 
 
### PhaseTransitioned(`PalletElectionProviderMultiPhasePhase`, `PalletElectionProviderMultiPhasePhase`, `u32`)
- **interface**: `api.events.electionProviderMultiPhase.PhaseTransitioned.is`
- **summary**:    There was a phase transition in a given round. 
 
### Rewarded(`AccountId32`, `u128`)
- **interface**: `api.events.electionProviderMultiPhase.Rewarded.is`
- **summary**:    An account has been rewarded for their signed submission being finalized. 
 
### Slashed(`AccountId32`, `u128`)
- **interface**: `api.events.electionProviderMultiPhase.Slashed.is`
- **summary**:    An account has been slashed for submitting an invalid signed submission. 
 
### SolutionStored(`PalletElectionProviderMultiPhaseElectionCompute`, `Option<AccountId32>`, `bool`)
- **interface**: `api.events.electionProviderMultiPhase.SolutionStored.is`
- **summary**:    A solution was stored with the given compute. 

   The `origin` indicates the origin of the solution. If `origin` is `Some(AccountId)`,  the stored solution was submited in the signed phase by a miner with the `AccountId`.  Otherwise, the solution was stored either during the unsigned phase or by  `T::ForceOrigin`. The `bool` is `true` when a previous solution was ejected to make  room for this one. 

___


## grandpa
 
### NewAuthorities(`Vec<(SpConsensusGrandpaAppPublic,u64)>`)
- **interface**: `api.events.grandpa.NewAuthorities.is`
- **summary**:    New authority set has been applied. 
 
### Paused()
- **interface**: `api.events.grandpa.Paused.is`
- **summary**:    Current authority set has been paused. 
 
### Resumed()
- **interface**: `api.events.grandpa.Resumed.is`
- **summary**:    Current authority set has been resumed. 

___


## imOnline
 
### AllGood()
- **interface**: `api.events.imOnline.AllGood.is`
- **summary**:    At the end of the session, no offence was committed. 
 
### HeartbeatReceived(`PalletImOnlineSr25519AppSr25519Public`)
- **interface**: `api.events.imOnline.HeartbeatReceived.is`
- **summary**:    A new heartbeat was received from `AuthorityId`. 
 
### SomeOffline(`Vec<(AccountId32,PalletStakingExposure)>`)
- **interface**: `api.events.imOnline.SomeOffline.is`
- **summary**:    At the end of the session, at least one validator was found to be offline. 

___


## offences
 
### Offence(`[u8;16]`, `Bytes`)
- **interface**: `api.events.offences.Offence.is`
- **summary**:    There is an offence reported of the given `kind` happened at the `session_index` and  (kind-specific) time slot. This event is not deposited for duplicate slashes.  \[kind, timeslot\]. 

___


## palletProxy
 
### ProxyRemoved(`AccountId32`)
- **interface**: `api.events.palletProxy.ProxyRemoved.is`
- **summary**:    Proxy account removed 
 
### ProxyStored(`AccountId32`)
- **interface**: `api.events.palletProxy.ProxyStored.is`
- **summary**:    The record account that uniquely identify 
 
### ProxySuspended(`AccountId32`)
- **interface**: `api.events.palletProxy.ProxySuspended.is`
- **summary**:    Proxy account suspended 
 
### ProxyUpdated(`AccountId32`)
- **interface**: `api.events.palletProxy.ProxyUpdated.is`
- **summary**:    The updated proxy account 
 
### TokenUsageExceed(`AccountId32`)
- **interface**: `api.events.palletProxy.TokenUsageExceed.is`
- **summary**:    allowed token usage exceed. 

___


## session
 
### NewSession(`u32`)
- **interface**: `api.events.session.NewSession.is`
- **summary**:    New session has happened. Note that the argument is the session index, not the  block number as the type might suggest. 

___


## staking
 
### Bonded(`AccountId32`, `u128`)
- **interface**: `api.events.staking.Bonded.is`
- **summary**:    An account has bonded this amount. \[stash, amount\] 

   NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,  it will not be emitted for staking rewards when they are added to stake. 
 
### Chilled(`AccountId32`)
- **interface**: `api.events.staking.Chilled.is`
- **summary**:    An account has stopped participating as either a validator or nominator. 
 
### EraPaid(`u32`, `u128`, `u128`)
- **interface**: `api.events.staking.EraPaid.is`
- **summary**:    The era payout has been set; the first balance is the validator-payout; the second is  the remainder from the maximum amount of reward. 
 
### ForceEra(`PalletStakingForcing`)
- **interface**: `api.events.staking.ForceEra.is`
- **summary**:    A new force era mode was set. 
 
### Kicked(`AccountId32`, `AccountId32`)
- **interface**: `api.events.staking.Kicked.is`
- **summary**:    A nominator has been kicked from a validator. 
 
### OldSlashingReportDiscarded(`u32`)
- **interface**: `api.events.staking.OldSlashingReportDiscarded.is`
- **summary**:    An old slashing report from a prior era was discarded because it could  not be processed. 
 
### PayoutStarted(`u32`, `AccountId32`)
- **interface**: `api.events.staking.PayoutStarted.is`
- **summary**:    The stakers' rewards are getting paid. 
 
### Rewarded(`AccountId32`, `u128`)
- **interface**: `api.events.staking.Rewarded.is`
- **summary**:    The nominator has been rewarded by this amount. 
 
### Slashed(`AccountId32`, `u128`)
- **interface**: `api.events.staking.Slashed.is`
- **summary**:    A staker (validator or nominator) has been slashed by the given amount. 
 
### SlashReported(`AccountId32`, `Perbill`, `u32`)
- **interface**: `api.events.staking.SlashReported.is`
- **summary**:    A slash for the given validator, for the given percentage of their stake, at the given  era as been reported. 
 
### StakersElected()
- **interface**: `api.events.staking.StakersElected.is`
- **summary**:    A new set of stakers was elected. 
 
### StakingElectionFailed()
- **interface**: `api.events.staking.StakingElectionFailed.is`
- **summary**:    The election failed. No new era is planned. 
 
### Unbonded(`AccountId32`, `u128`)
- **interface**: `api.events.staking.Unbonded.is`
- **summary**:    An account has unbonded this amount. 
 
### ValidatorPrefsSet(`AccountId32`, `PalletStakingValidatorPrefs`)
- **interface**: `api.events.staking.ValidatorPrefsSet.is`
- **summary**:    A validator has set their preferences. 
 
### Withdrawn(`AccountId32`, `u128`)
- **interface**: `api.events.staking.Withdrawn.is`
- **summary**:    An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`  from the unlocking queue. 

___


## sudo
 
### KeyChanged(`Option<AccountId32>`)
- **interface**: `api.events.sudo.KeyChanged.is`
- **summary**:    The \[sudoer\] just switched identity; the old key is supplied if one existed. 
 
### Sudid(`Result<Null, SpRuntimeDispatchError>`)
- **interface**: `api.events.sudo.Sudid.is`
- **summary**:    A sudo just took place. \[result\] 
 
### SudoAsDone(`Result<Null, SpRuntimeDispatchError>`)
- **interface**: `api.events.sudo.SudoAsDone.is`
- **summary**:    A sudo just took place. \[result\] 

___


## system
 
### CodeUpdated()
- **interface**: `api.events.system.CodeUpdated.is`
- **summary**:    `:code` was updated. 
 
### ExtrinsicFailed(`SpRuntimeDispatchError`, `FrameSupportDispatchDispatchInfo`)
- **interface**: `api.events.system.ExtrinsicFailed.is`
- **summary**:    An extrinsic failed. 
 
### ExtrinsicSuccess(`FrameSupportDispatchDispatchInfo`)
- **interface**: `api.events.system.ExtrinsicSuccess.is`
- **summary**:    An extrinsic completed successfully. 
 
### KilledAccount(`AccountId32`)
- **interface**: `api.events.system.KilledAccount.is`
- **summary**:    An account was reaped. 
 
### NewAccount(`AccountId32`)
- **interface**: `api.events.system.NewAccount.is`
- **summary**:    A new account was created. 
 
### Remarked(`AccountId32`, `H256`)
- **interface**: `api.events.system.Remarked.is`
- **summary**:    On on-chain remark happened. 

___


## taskMeta
 
### AlreadyExist(`u64`)
- **interface**: `api.events.taskMeta.AlreadyExist.is`
- **summary**:    Already exist case 
 
### ColAlreadyExist(`Text`)
- **interface**: `api.events.taskMeta.ColAlreadyExist.is`
- **summary**:    Already exist case 
 
### ColMetaStored(`Text`)
- **interface**: `api.events.taskMeta.ColMetaStored.is`
- **summary**:    Collections 
 
### PayableTaskMetaAlreadyExist(`u64`)
- **interface**: `api.events.taskMeta.PayableTaskMetaAlreadyExist.is`
 
### PayableTaskMetaStorage(`u64`)
- **interface**: `api.events.taskMeta.PayableTaskMetaStorage.is`
 
### TaskMetaStored(`u64`)
- **interface**: `api.events.taskMeta.TaskMetaStored.is`
- **summary**:    the record id that uniquely identify 

___


## taskSchedule
 
### AlreadyExist(`u64`)
- **interface**: `api.events.taskSchedule.AlreadyExist.is`
- **summary**:    Already exist case 
 
### PayableScheduleStored(`u64`)
- **interface**: `api.events.taskSchedule.PayableScheduleStored.is`
- **summary**:    Payable Schedule 
 
### RewardIndexer(`AccountId32`, `u128`)
- **interface**: `api.events.taskSchedule.RewardIndexer.is`
- **summary**:    Reward indexer 
 
### ScheduleStored(`u64`)
- **interface**: `api.events.taskSchedule.ScheduleStored.is`
- **summary**:    the record id that uniquely identify 
 
### ScheduleUpdated(`u64`)
- **interface**: `api.events.taskSchedule.ScheduleUpdated.is`
- **summary**:    Updated Schedule 

___


## tesseractSigStorage
 
### ChronicleRegistered(`AccountId32`, `AccountId32`)
- **interface**: `api.events.tesseractSigStorage.ChronicleRegistered.is`
- **summary**:    Chronicle has ben registered  .0 TimeId  .1 Validator's AccountId 
 
### NewTssGroupKey(`u64`, `[u8;33]`)
- **interface**: `api.events.tesseractSigStorage.NewTssGroupKey.is`
- **summary**:    New group key submitted to runtime  .0 - ShardId  .1 - group key bytes 
 
### OffenceCommitted(`AccountId32`, `u8`)
- **interface**: `api.events.tesseractSigStorage.OffenceCommitted.is`
- **summary**:    Offence reported, above threshold s.t.  reports are moved from reported to committed.  .0 Offender TimeId  .1 Report count 
 
### OffenceReported(`AccountId32`, `u8`)
- **interface**: `api.events.tesseractSigStorage.OffenceReported.is`
- **summary**:    Offence reported  .0 Offender TimeId  .1 Report count 
 
### ShardOffline(`u64`)
- **interface**: `api.events.tesseractSigStorage.ShardOffline.is`
- **summary**:    Shard went offline due to committed offenses preventing threshold  or task execution timeout(s) by collector  .0 ShardId 
 
### ShardRegistered(`u64`)
- **interface**: `api.events.tesseractSigStorage.ShardRegistered.is`
- **summary**:    Shard has ben registered with new Id  .0 ShardId 
 
### SignatureStored(`u64`, `u64`)
- **interface**: `api.events.tesseractSigStorage.SignatureStored.is`
- **summary**:    The event data for stored signature  the signature id that uniquely identify the signature 
 
### TaskExecutionTimeout(`u64`)
- **interface**: `api.events.tesseractSigStorage.TaskExecutionTimeout.is`
- **summary**:    Task execution timed out for task 

___


## transactionPayment
 
### TransactionFeePaid(`AccountId32`, `u128`, `u128`)
- **interface**: `api.events.transactionPayment.TransactionFeePaid.is`
- **summary**:    A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,  has been paid by `who`. 

___


## treasury
 
### Awarded(`u32`, `u128`, `AccountId32`)
- **interface**: `api.events.treasury.Awarded.is`
- **summary**:    Some funds have been allocated. 
 
### Burnt(`u128`)
- **interface**: `api.events.treasury.Burnt.is`
- **summary**:    Some of our funds have been burnt. 
 
### Deposit(`u128`)
- **interface**: `api.events.treasury.Deposit.is`
- **summary**:    Some funds have been deposited. 
 
### Proposed(`u32`)
- **interface**: `api.events.treasury.Proposed.is`
- **summary**:    New proposal. 
 
### Rejected(`u32`, `u128`)
- **interface**: `api.events.treasury.Rejected.is`
- **summary**:    A proposal was rejected; funds were slashed. 
 
### Rollover(`u128`)
- **interface**: `api.events.treasury.Rollover.is`
- **summary**:    Spending has finished; this is the amount that rolls over until next spend. 
 
### SpendApproved(`u32`, `u128`, `AccountId32`)
- **interface**: `api.events.treasury.SpendApproved.is`
- **summary**:    A new spend proposal has been approved. 
 
### Spending(`u128`)
- **interface**: `api.events.treasury.Spending.is`
- **summary**:    We have ended a spend period and will now allocate funds. 
 
### UpdatedInactive(`u128`, `u128`)
- **interface**: `api.events.treasury.UpdatedInactive.is`
- **summary**:    The inactive funds of the pallet have been updated. 

___


## utility
 
### BatchCompleted()
- **interface**: `api.events.utility.BatchCompleted.is`
- **summary**:    Batch of dispatches completed fully with no error. 
 
### BatchCompletedWithErrors()
- **interface**: `api.events.utility.BatchCompletedWithErrors.is`
- **summary**:    Batch of dispatches completed but has errors. 
 
### BatchInterrupted(`u32`, `SpRuntimeDispatchError`)
- **interface**: `api.events.utility.BatchInterrupted.is`
- **summary**:    Batch of dispatches did not complete fully. Index of first failing dispatch given, as  well as the error. 
 
### DispatchedAs(`Result<Null, SpRuntimeDispatchError>`)
- **interface**: `api.events.utility.DispatchedAs.is`
- **summary**:    A call was dispatched. 
 
### ItemCompleted()
- **interface**: `api.events.utility.ItemCompleted.is`
- **summary**:    A single item within a Batch of dispatches has completed with no error. 
 
### ItemFailed(`SpRuntimeDispatchError`)
- **interface**: `api.events.utility.ItemFailed.is`
- **summary**:    A single item within a Batch of dispatches has completed with error. 

___


## vesting
 
### Claimed(`AccountId32`, `u128`)
- **interface**: `api.events.vesting.Claimed.is`
- **summary**:    Claimed vesting. 
 
### VestingScheduleAdded(`AccountId32`, `AccountId32`, `AnalogVestingVestingSchedule`)
- **interface**: `api.events.vesting.VestingScheduleAdded.is`
- **summary**:    Added new vesting schedule. 
 
### VestingSchedulesUpdated(`AccountId32`)
- **interface**: `api.events.vesting.VestingSchedulesUpdated.is`
- **summary**:    Updated vesting schedules. 

___


## voterList
 
### Rebagged(`AccountId32`, `u64`, `u64`)
- **interface**: `api.events.voterList.Rebagged.is`
- **summary**:    Moved an account from one bag to another. 
 
### ScoreUpdated(`AccountId32`, `u64`)
- **interface**: `api.events.voterList.ScoreUpdated.is`
- **summary**:    Updated the score of some account to the given amount. 
