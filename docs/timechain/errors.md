---
title: Errors
---

This page lists the errors that can be encountered in the different modules. 

(NOTE: These were generated from a static/snapshot view of a recent default Timechain runtime. Some items may not be available in older nodes, or in any customized implementations.)

- **[babe](#babe)**

- **[balances](#balances)**

- **[council](#council)**

- **[electionProviderMultiPhase](#electionprovidermultiphase)**

- **[grandpa](#grandpa)**

- **[imOnline](#imonline)**

- **[palletProxy](#palletproxy)**

- **[session](#session)**

- **[staking](#staking)**

- **[sudo](#sudo)**

- **[system](#system)**

- **[taskMeta](#taskmeta)**

- **[taskSchedule](#taskschedule)**

- **[tesseractSigStorage](#tesseractsigstorage)**

- **[treasury](#treasury)**

- **[utility](#utility)**

- **[vesting](#vesting)**

- **[voterList](#voterlist)**


___


## babe
 
### DuplicateOffenceReport
- **interface**: `api.errors.babe.DuplicateOffenceReport.is`
- **summary**:    A given equivocation report is valid but already previously reported. 
 
### InvalidConfiguration
- **interface**: `api.errors.babe.InvalidConfiguration.is`
- **summary**:    Submitted configuration is invalid. 
 
### InvalidEquivocationProof
- **interface**: `api.errors.babe.InvalidEquivocationProof.is`
- **summary**:    An equivocation proof provided as part of an equivocation report is invalid. 
 
### InvalidKeyOwnershipProof
- **interface**: `api.errors.babe.InvalidKeyOwnershipProof.is`
- **summary**:    A key ownership proof provided as part of an equivocation report is invalid. 

___


## balances
 
### DeadAccount
- **interface**: `api.errors.balances.DeadAccount.is`
- **summary**:    Beneficiary account must pre-exist. 
 
### ExistentialDeposit
- **interface**: `api.errors.balances.ExistentialDeposit.is`
- **summary**:    Value too low to create account due to existential deposit. 
 
### ExistingVestingSchedule
- **interface**: `api.errors.balances.ExistingVestingSchedule.is`
- **summary**:    A vesting schedule already exists for this account. 
 
### Expendability
- **interface**: `api.errors.balances.Expendability.is`
- **summary**:    Transfer/payment would kill account. 
 
### InsufficientBalance
- **interface**: `api.errors.balances.InsufficientBalance.is`
- **summary**:    Balance too low to send value. 
 
### LiquidityRestrictions
- **interface**: `api.errors.balances.LiquidityRestrictions.is`
- **summary**:    Account liquidity restrictions prevent withdrawal. 
 
### TooManyFreezes
- **interface**: `api.errors.balances.TooManyFreezes.is`
- **summary**:    Number of freezes exceed `MaxFreezes`. 
 
### TooManyHolds
- **interface**: `api.errors.balances.TooManyHolds.is`
- **summary**:    Number of holds exceed `MaxHolds`. 
 
### TooManyReserves
- **interface**: `api.errors.balances.TooManyReserves.is`
- **summary**:    Number of named reserves exceed `MaxReserves`. 
 
### VestingBalance
- **interface**: `api.errors.balances.VestingBalance.is`
- **summary**:    Vesting balance too high to send value. 

___


## council
 
### AlreadyInitialized
- **interface**: `api.errors.council.AlreadyInitialized.is`
- **summary**:    Members are already initialized! 
 
### DuplicateProposal
- **interface**: `api.errors.council.DuplicateProposal.is`
- **summary**:    Duplicate proposals not allowed 
 
### DuplicateVote
- **interface**: `api.errors.council.DuplicateVote.is`
- **summary**:    Duplicate vote ignored 
 
### NotMember
- **interface**: `api.errors.council.NotMember.is`
- **summary**:    Account is not a member 
 
### ProposalMissing
- **interface**: `api.errors.council.ProposalMissing.is`
- **summary**:    Proposal must exist 
 
### TooEarly
- **interface**: `api.errors.council.TooEarly.is`
- **summary**:    The close call was made too early, before the end of the voting. 
 
### TooManyProposals
- **interface**: `api.errors.council.TooManyProposals.is`
- **summary**:    There can only be a maximum of `MaxProposals` active proposals. 
 
### WrongIndex
- **interface**: `api.errors.council.WrongIndex.is`
- **summary**:    Mismatched index 
 
### WrongProposalLength
- **interface**: `api.errors.council.WrongProposalLength.is`
- **summary**:    The given length bound for the proposal was too low. 
 
### WrongProposalWeight
- **interface**: `api.errors.council.WrongProposalWeight.is`
- **summary**:    The given weight bound for the proposal was too low. 

___


## electionProviderMultiPhase
 
### BoundNotMet
- **interface**: `api.errors.electionProviderMultiPhase.BoundNotMet.is`
- **summary**:    Some bound not met 
 
### CallNotAllowed
- **interface**: `api.errors.electionProviderMultiPhase.CallNotAllowed.is`
- **summary**:    The call is not allowed at this point. 
 
### FallbackFailed
- **interface**: `api.errors.electionProviderMultiPhase.FallbackFailed.is`
- **summary**:    The fallback failed 
 
### InvalidSubmissionIndex
- **interface**: `api.errors.electionProviderMultiPhase.InvalidSubmissionIndex.is`
- **summary**:    `Self::insert_submission` returned an invalid index. 
 
### MissingSnapshotMetadata
- **interface**: `api.errors.electionProviderMultiPhase.MissingSnapshotMetadata.is`
- **summary**:    Snapshot metadata should exist but didn't. 
 
### OcwCallWrongEra
- **interface**: `api.errors.electionProviderMultiPhase.OcwCallWrongEra.is`
- **summary**:    OCW submitted solution for wrong round 
 
### PreDispatchEarlySubmission
- **interface**: `api.errors.electionProviderMultiPhase.PreDispatchEarlySubmission.is`
- **summary**:    Submission was too early. 
 
### PreDispatchWeakSubmission
- **interface**: `api.errors.electionProviderMultiPhase.PreDispatchWeakSubmission.is`
- **summary**:    Submission was too weak, score-wise. 
 
### PreDispatchWrongWinnerCount
- **interface**: `api.errors.electionProviderMultiPhase.PreDispatchWrongWinnerCount.is`
- **summary**:    Wrong number of winners presented. 
 
### SignedCannotPayDeposit
- **interface**: `api.errors.electionProviderMultiPhase.SignedCannotPayDeposit.is`
- **summary**:    The origin failed to pay the deposit. 
 
### SignedInvalidWitness
- **interface**: `api.errors.electionProviderMultiPhase.SignedInvalidWitness.is`
- **summary**:    Witness data to dispatchable is invalid. 
 
### SignedQueueFull
- **interface**: `api.errors.electionProviderMultiPhase.SignedQueueFull.is`
- **summary**:    The queue was full, and the solution was not better than any of the existing ones. 
 
### SignedTooMuchWeight
- **interface**: `api.errors.electionProviderMultiPhase.SignedTooMuchWeight.is`
- **summary**:    The signed submission consumes too much weight 
 
### TooManyWinners
- **interface**: `api.errors.electionProviderMultiPhase.TooManyWinners.is`
- **summary**:    Submitted solution has too many winners 

___


## grandpa
 
### ChangePending
- **interface**: `api.errors.grandpa.ChangePending.is`
- **summary**:    Attempt to signal GRANDPA change with one already pending. 
 
### DuplicateOffenceReport
- **interface**: `api.errors.grandpa.DuplicateOffenceReport.is`
- **summary**:    A given equivocation report is valid but already previously reported. 
 
### InvalidEquivocationProof
- **interface**: `api.errors.grandpa.InvalidEquivocationProof.is`
- **summary**:    An equivocation proof provided as part of an equivocation report is invalid. 
 
### InvalidKeyOwnershipProof
- **interface**: `api.errors.grandpa.InvalidKeyOwnershipProof.is`
- **summary**:    A key ownership proof provided as part of an equivocation report is invalid. 
 
### PauseFailed
- **interface**: `api.errors.grandpa.PauseFailed.is`
- **summary**:    Attempt to signal GRANDPA pause when the authority set isn't live  (either paused or already pending pause). 
 
### ResumeFailed
- **interface**: `api.errors.grandpa.ResumeFailed.is`
- **summary**:    Attempt to signal GRANDPA resume when the authority set isn't paused  (either live or already pending resume). 
 
### TooSoon
- **interface**: `api.errors.grandpa.TooSoon.is`
- **summary**:    Cannot signal forced change so soon after last. 

___


## imOnline
 
### DuplicatedHeartbeat
- **interface**: `api.errors.imOnline.DuplicatedHeartbeat.is`
- **summary**:    Duplicated heartbeat. 
 
### InvalidKey
- **interface**: `api.errors.imOnline.InvalidKey.is`
- **summary**:    Non existent public key. 

___


## palletProxy
 
### ErrorRef
- **interface**: `api.errors.palletProxy.ErrorRef.is`
- **summary**:    Error getting schedule ref. 
 
### NoPermission
- **interface**: `api.errors.palletProxy.NoPermission.is`
- **summary**:    The signing account has no permission to do the operation. 
 
### ProxyAlreadyExists
- **interface**: `api.errors.palletProxy.ProxyAlreadyExists.is`
- **summary**:    Cannot add proxy that already exists 
 
### ProxyNotExist
- **interface**: `api.errors.palletProxy.ProxyNotExist.is`
- **summary**:    Cannot remove proxy that does not exist 

___


## session
 
### DuplicatedKey
- **interface**: `api.errors.session.DuplicatedKey.is`
- **summary**:    Registered duplicate key. 
 
### InvalidProof
- **interface**: `api.errors.session.InvalidProof.is`
- **summary**:    Invalid ownership proof. 
 
### NoAccount
- **interface**: `api.errors.session.NoAccount.is`
- **summary**:    Key setting account is not live, so it's impossible to associate keys. 
 
### NoAssociatedValidatorId
- **interface**: `api.errors.session.NoAssociatedValidatorId.is`
- **summary**:    No associated validator ID for account. 
 
### NoKeys
- **interface**: `api.errors.session.NoKeys.is`
- **summary**:    No keys are associated with this account. 

___


## staking
 
### AlreadyBonded
- **interface**: `api.errors.staking.AlreadyBonded.is`
- **summary**:    Stash is already bonded. 
 
### AlreadyClaimed
- **interface**: `api.errors.staking.AlreadyClaimed.is`
- **summary**:    Rewards for this era have already been claimed for this validator. 
 
### AlreadyPaired
- **interface**: `api.errors.staking.AlreadyPaired.is`
- **summary**:    Controller is already paired. 
 
### BadState
- **interface**: `api.errors.staking.BadState.is`
- **summary**:    Internal state has become somehow corrupted and the operation cannot continue. 
 
### BadTarget
- **interface**: `api.errors.staking.BadTarget.is`
- **summary**:    A nomination target was supplied that was blocked or otherwise not a validator. 
 
### BoundNotMet
- **interface**: `api.errors.staking.BoundNotMet.is`
- **summary**:    Some bound is not met. 
 
### CannotChillOther
- **interface**: `api.errors.staking.CannotChillOther.is`
- **summary**:    The user has enough bond and thus cannot be chilled forcefully by an external person. 
 
### CommissionTooLow
- **interface**: `api.errors.staking.CommissionTooLow.is`
- **summary**:    Commission is too low. Must be at least `MinCommission`. 
 
### DuplicateIndex
- **interface**: `api.errors.staking.DuplicateIndex.is`
- **summary**:    Duplicate index. 
 
### EmptyTargets
- **interface**: `api.errors.staking.EmptyTargets.is`
- **summary**:    Targets cannot be empty. 
 
### FundedTarget
- **interface**: `api.errors.staking.FundedTarget.is`
- **summary**:    Attempting to target a stash that still has funds. 
 
### IncorrectHistoryDepth
- **interface**: `api.errors.staking.IncorrectHistoryDepth.is`
- **summary**:    Incorrect previous history depth input provided. 
 
### IncorrectSlashingSpans
- **interface**: `api.errors.staking.IncorrectSlashingSpans.is`
- **summary**:    Incorrect number of slashing spans provided. 
 
### InsufficientBond
- **interface**: `api.errors.staking.InsufficientBond.is`
- **summary**:    Cannot have a validator or nominator role, with value less than the minimum defined by  governance (see `MinValidatorBond` and `MinNominatorBond`). If unbonding is the  intention, `chill` first to remove one's role as validator/nominator. 
 
### InvalidEraToReward
- **interface**: `api.errors.staking.InvalidEraToReward.is`
- **summary**:    Invalid era to reward. 
 
### InvalidNumberOfNominations
- **interface**: `api.errors.staking.InvalidNumberOfNominations.is`
- **summary**:    Invalid number of nominations. 
 
### InvalidSlashIndex
- **interface**: `api.errors.staking.InvalidSlashIndex.is`
- **summary**:    Slash record index out of bounds. 
 
### NoMoreChunks
- **interface**: `api.errors.staking.NoMoreChunks.is`
- **summary**:    Can not schedule more unlock chunks. 
 
### NotController
- **interface**: `api.errors.staking.NotController.is`
- **summary**:    Not a controller account. 
 
### NotSortedAndUnique
- **interface**: `api.errors.staking.NotSortedAndUnique.is`
- **summary**:    Items are not sorted and unique. 
 
### NotStash
- **interface**: `api.errors.staking.NotStash.is`
- **summary**:    Not a stash account. 
 
### NoUnlockChunk
- **interface**: `api.errors.staking.NoUnlockChunk.is`
- **summary**:    Can not rebond without unlocking chunks. 
 
### TooManyNominators
- **interface**: `api.errors.staking.TooManyNominators.is`
- **summary**:    There are too many nominators in the system. Governance needs to adjust the staking  settings to keep things safe for the runtime. 
 
### TooManyTargets
- **interface**: `api.errors.staking.TooManyTargets.is`
- **summary**:    Too many nomination targets supplied. 
 
### TooManyValidators
- **interface**: `api.errors.staking.TooManyValidators.is`
- **summary**:    There are too many validator candidates in the system. Governance needs to adjust the  staking settings to keep things safe for the runtime. 

___


## sudo
 
### RequireSudo
- **interface**: `api.errors.sudo.RequireSudo.is`
- **summary**:    Sender must be the Sudo account 

___


## system
 
### CallFiltered
- **interface**: `api.errors.system.CallFiltered.is`
- **summary**:    The origin filter prevent the call to be dispatched. 
 
### FailedToExtractRuntimeVersion
- **interface**: `api.errors.system.FailedToExtractRuntimeVersion.is`
- **summary**:    Failed to extract the runtime version from the new runtime. 

   Either calling `Core_version` or decoding `RuntimeVersion` failed. 
 
### InvalidSpecName
- **interface**: `api.errors.system.InvalidSpecName.is`
- **summary**:    The name of specification does not match between the current runtime  and the new runtime. 
 
### NonDefaultComposite
- **interface**: `api.errors.system.NonDefaultComposite.is`
- **summary**:    Suicide called when the account has non-default composite data. 
 
### NonZeroRefCount
- **interface**: `api.errors.system.NonZeroRefCount.is`
- **summary**:    There is a non-zero reference count preventing the account from being purged. 
 
### SpecVersionNeedsToIncrease
- **interface**: `api.errors.system.SpecVersionNeedsToIncrease.is`
- **summary**:    The specification version is not allowed to decrease between the current runtime  and the new runtime. 

___


## taskMeta
 
### ErrorRef
- **interface**: `api.errors.taskMeta.ErrorRef.is`
- **summary**:    Error getting schedule ref. 
 
### NotProxyAccount
- **interface**: `api.errors.taskMeta.NotProxyAccount.is`
- **summary**:    Not a valid submitter 

___


## taskSchedule
 
### ErrorRef
- **interface**: `api.errors.taskSchedule.ErrorRef.is`
- **summary**:    Error getting schedule ref. 
 
### NoLocalAcctForSignedTx
- **interface**: `api.errors.taskSchedule.NoLocalAcctForSignedTx.is`
- **summary**:    no local account for signed tx 
 
### NoPermission
- **interface**: `api.errors.taskSchedule.NoPermission.is`
- **summary**:    The signing account has no permission to do the operation. 
 
### NotProxyAccount
- **interface**: `api.errors.taskSchedule.NotProxyAccount.is`
- **summary**:    Not a valid submitter 
 
### OffchainSignedTxFailed
- **interface**: `api.errors.taskSchedule.OffchainSignedTxFailed.is`
- **summary**:    Offchain signed tx failed 
 
### ProxyNotUpdated
- **interface**: `api.errors.taskSchedule.ProxyNotUpdated.is`
- **summary**:    Proxy account(s) token usage not updated 
 
### ShardNotEligibleForTasks
- **interface**: `api.errors.taskSchedule.ShardNotEligibleForTasks.is`
- **summary**:    Shard cannot be assigned tasks due to ineligibility 
 
### TaskMetadataNotRegistered
- **interface**: `api.errors.taskSchedule.TaskMetadataNotRegistered.is`
- **summary**:    Task Metadata is not registered 

___


## tesseractSigStorage
 
### AlreadyCollector
- **interface**: `api.errors.tesseractSigStorage.AlreadyCollector.is`
- **summary**:    Cannot set collector if they are already in that role 
 
### ChronicleAlreadyInSet
- **interface**: `api.errors.tesseractSigStorage.ChronicleAlreadyInSet.is`
- **summary**:    Chronicle already in set 
 
### ChronicleAlreadyRegistered
- **interface**: `api.errors.tesseractSigStorage.ChronicleAlreadyRegistered.is`
- **summary**:    Chronicle already registered 
 
### ChronicleNotRegistered
- **interface**: `api.errors.tesseractSigStorage.ChronicleNotRegistered.is`
- **summary**:    Chronicle not registered 
 
### ChronicleSetIsFull
- **interface**: `api.errors.tesseractSigStorage.ChronicleSetIsFull.is`
- **summary**:    Chronicle set is full 
 
### CollectorIndexBeyondMemberLen
- **interface**: `api.errors.tesseractSigStorage.CollectorIndexBeyondMemberLen.is`
- **summary**:    Collector index exceeds length of members 
 
### DefaultAccountForbidden
- **interface**: `api.errors.tesseractSigStorage.DefaultAccountForbidden.is`
- **summary**:    Default account is not allowed for this operation 
 
### DuplicateShardMembersNotAllowed
- **interface**: `api.errors.tesseractSigStorage.DuplicateShardMembersNotAllowed.is`
 
### DuplicateSignature
- **interface**: `api.errors.tesseractSigStorage.DuplicateSignature.is`
- **summary**:    TSS Signature already added 
 
### EncodedAccountWrongLen
- **interface**: `api.errors.tesseractSigStorage.EncodedAccountWrongLen.is`
- **summary**:    Encoded account wrong length 
 
### FailedToGetValidatorId
- **interface**: `api.errors.tesseractSigStorage.FailedToGetValidatorId.is`
- **summary**:    Failed to get validator id 
 
### InvalidCaller
- **interface**: `api.errors.tesseractSigStorage.InvalidCaller.is`
- **summary**:    Invalid Caller, 
 
### InvalidReporterId
- **interface**: `api.errors.tesseractSigStorage.InvalidReporterId.is`
- **summary**:    Reporter TimeId can not be converted to Public key 
 
### InvalidValidationSignature
- **interface**: `api.errors.tesseractSigStorage.InvalidValidationSignature.is`
- **summary**:    Invalid validation signature 
 
### NoLocalAcctForSignedTx
- **interface**: `api.errors.tesseractSigStorage.NoLocalAcctForSignedTx.is`
- **summary**:    no local account for signed tx 
 
### OffchainSignedTxFailed
- **interface**: `api.errors.tesseractSigStorage.OffchainSignedTxFailed.is`
- **summary**:    Offchain signed tx failed 
 
### OffenderNotInMembers
- **interface**: `api.errors.tesseractSigStorage.OffenderNotInMembers.is`
- **summary**:    Offender not in members 
 
### OnlyCallableByCollector
- **interface**: `api.errors.tesseractSigStorage.OnlyCallableByCollector.is`
- **summary**:    Caller is not shard's collector so cannot call this function 
 
### OnlyValidatorCanRegisterChronicle
- **interface**: `api.errors.tesseractSigStorage.OnlyValidatorCanRegisterChronicle.is`
- **summary**:    Only validator can register chronicle 
 
### ProofVerificationFailed
- **interface**: `api.errors.tesseractSigStorage.ProofVerificationFailed.is`
- **summary**:    Misbehavior report proof verification failed 
 
### ShardAlreadyOffline
- **interface**: `api.errors.tesseractSigStorage.ShardAlreadyOffline.is`
- **summary**:    Shard status is offline now 
 
### ShardIdOverflow
- **interface**: `api.errors.tesseractSigStorage.ShardIdOverflow.is`
- **summary**:    ShardId generation overflowed u64 type 
 
### ShardIsNotRegistered
- **interface**: `api.errors.tesseractSigStorage.ShardIsNotRegistered.is`
- **summary**:    Shard does not exist in storage 
 
### TaskNotScheduled
- **interface**: `api.errors.tesseractSigStorage.TaskNotScheduled.is`
- **summary**:    Task not scheduled 
 
### UnregisteredWorkerDataSubmission
- **interface**: `api.errors.tesseractSigStorage.UnregisteredWorkerDataSubmission.is`
- **summary**:    Unauthorized attempt to add signed data 
 
### UnsupportedMembershipSize
- **interface**: `api.errors.tesseractSigStorage.UnsupportedMembershipSize.is`
- **summary**:    Shard registartion failed because wrong number of members  NOTE: supported sizes are 3, 5, and 10 

___


## treasury
 
### InsufficientPermission
- **interface**: `api.errors.treasury.InsufficientPermission.is`
- **summary**:    The spend origin is valid but the amount it is allowed to spend is lower than the  amount to be spent. 
 
### InsufficientProposersBalance
- **interface**: `api.errors.treasury.InsufficientProposersBalance.is`
- **summary**:    Proposer's balance is too low. 
 
### InvalidIndex
- **interface**: `api.errors.treasury.InvalidIndex.is`
- **summary**:    No proposal or bounty at that index. 
 
### ProposalNotApproved
- **interface**: `api.errors.treasury.ProposalNotApproved.is`
- **summary**:    Proposal has not been approved. 
 
### TooManyApprovals
- **interface**: `api.errors.treasury.TooManyApprovals.is`
- **summary**:    Too many approvals in the queue. 

___


## utility
 
### TooManyCalls
- **interface**: `api.errors.utility.TooManyCalls.is`
- **summary**:    Too many calls batched. 

___


## vesting
 
### AmountLow
- **interface**: `api.errors.vesting.AmountLow.is`
- **summary**:    The vested transfer amount is too low 
 
### InsufficientBalanceToLock
- **interface**: `api.errors.vesting.InsufficientBalanceToLock.is`
- **summary**:    Insufficient amount of balance to lock 
 
### MaxVestingSchedulesExceeded
- **interface**: `api.errors.vesting.MaxVestingSchedulesExceeded.is`
- **summary**:    Failed because the maximum vesting schedules was exceeded 
 
### TooManyVestingSchedules
- **interface**: `api.errors.vesting.TooManyVestingSchedules.is`
- **summary**:    This account have too many vesting schedules 
 
### ZeroVestingPeriod
- **interface**: `api.errors.vesting.ZeroVestingPeriod.is`
- **summary**:    Vesting period is zero 
 
### ZeroVestingPeriodCount
- **interface**: `api.errors.vesting.ZeroVestingPeriodCount.is`
- **summary**:    Number of vests is zero 

___


## voterList
 
### List
- **interface**: `api.errors.voterList.List.is`
- **summary**:    A error in the list interface implementation. 
