---
title: Extrinsics
---

The following sections contain Extrinsics methods are part of the default Timechain runtime. On the api, these are exposed via `api.tx.<module>.<method>`. 

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

- **[timestamp](#timestamp)**

- **[treasury](#treasury)**

- **[utility](#utility)**

- **[vesting](#vesting)**

- **[voterList](#voterlist)**


___


## babe
 
### planConfigChange(config: `SpConsensusBabeDigestsNextConfigDescriptor`)
- **interface**: `api.tx.babe.planConfigChange`
- **summary**:    See [`Pallet::plan_config_change`]. 
 
### reportEquivocation(equivocation_proof: `SpConsensusSlotsEquivocationProof`, key_owner_proof: `SpSessionMembershipProof`)
- **interface**: `api.tx.babe.reportEquivocation`
- **summary**:    See [`Pallet::report_equivocation`]. 
 
### reportEquivocationUnsigned(equivocation_proof: `SpConsensusSlotsEquivocationProof`, key_owner_proof: `SpSessionMembershipProof`)
- **interface**: `api.tx.babe.reportEquivocationUnsigned`
- **summary**:    See [`Pallet::report_equivocation_unsigned`]. 

___


## balances
 
### forceSetBalance(who: `MultiAddress`, new_free: `Compact<u128>`)
- **interface**: `api.tx.balances.forceSetBalance`
- **summary**:    See [`Pallet::force_set_balance`]. 
 
### forceTransfer(source: `MultiAddress`, dest: `MultiAddress`, value: `Compact<u128>`)
- **interface**: `api.tx.balances.forceTransfer`
- **summary**:    See [`Pallet::force_transfer`]. 
 
### forceUnreserve(who: `MultiAddress`, amount: `u128`)
- **interface**: `api.tx.balances.forceUnreserve`
- **summary**:    See [`Pallet::force_unreserve`]. 
 
### setBalanceDeprecated(who: `MultiAddress`, new_free: `Compact<u128>`, old_reserved: `Compact<u128>`)
- **interface**: `api.tx.balances.setBalanceDeprecated`
- **summary**:    See [`Pallet::set_balance_deprecated`]. 
 
### transfer(dest: `MultiAddress`, value: `Compact<u128>`)
- **interface**: `api.tx.balances.transfer`
- **summary**:    See [`Pallet::transfer`]. 
 
### transferAll(dest: `MultiAddress`, keep_alive: `bool`)
- **interface**: `api.tx.balances.transferAll`
- **summary**:    See [`Pallet::transfer_all`]. 
 
### transferAllowDeath(dest: `MultiAddress`, value: `Compact<u128>`)
- **interface**: `api.tx.balances.transferAllowDeath`
- **summary**:    See [`Pallet::transfer_allow_death`]. 
 
### transferKeepAlive(dest: `MultiAddress`, value: `Compact<u128>`)
- **interface**: `api.tx.balances.transferKeepAlive`
- **summary**:    See [`Pallet::transfer_keep_alive`]. 
 
### upgradeAccounts(who: `Vec<AccountId32>`)
- **interface**: `api.tx.balances.upgradeAccounts`
- **summary**:    See [`Pallet::upgrade_accounts`]. 

___


## council
 
### close(proposal_hash: `H256`, index: `Compact<u32>`, proposal_weight_bound: `SpWeightsWeightV2Weight`, length_bound: `Compact<u32>`)
- **interface**: `api.tx.council.close`
- **summary**:    See [`Pallet::close`]. 
 
### disapproveProposal(proposal_hash: `H256`)
- **interface**: `api.tx.council.disapproveProposal`
- **summary**:    See [`Pallet::disapprove_proposal`]. 
 
### execute(proposal: `Call`, length_bound: `Compact<u32>`)
- **interface**: `api.tx.council.execute`
- **summary**:    See [`Pallet::execute`]. 
 
### propose(threshold: `Compact<u32>`, proposal: `Call`, length_bound: `Compact<u32>`)
- **interface**: `api.tx.council.propose`
- **summary**:    See [`Pallet::propose`]. 
 
### setMembers(new_members: `Vec<AccountId32>`, prime: `Option<AccountId32>`, old_count: `u32`)
- **interface**: `api.tx.council.setMembers`
- **summary**:    See [`Pallet::set_members`]. 
 
### vote(proposal: `H256`, index: `Compact<u32>`, approve: `bool`)
- **interface**: `api.tx.council.vote`
- **summary**:    See [`Pallet::vote`]. 

___


## electionProviderMultiPhase
 
### governanceFallback(maybe_max_voters: `Option<u32>`, maybe_max_targets: `Option<u32>`)
- **interface**: `api.tx.electionProviderMultiPhase.governanceFallback`
- **summary**:    See [`Pallet::governance_fallback`]. 
 
### setEmergencyElectionResult(supports: `Vec<(AccountId32,SpNposElectionsSupport)>`)
- **interface**: `api.tx.electionProviderMultiPhase.setEmergencyElectionResult`
- **summary**:    See [`Pallet::set_emergency_election_result`]. 
 
### setMinimumUntrustedScore(maybe_next_score: `Option<SpNposElectionsElectionScore>`)
- **interface**: `api.tx.electionProviderMultiPhase.setMinimumUntrustedScore`
- **summary**:    See [`Pallet::set_minimum_untrusted_score`]. 
 
### submit(raw_solution: `PalletElectionProviderMultiPhaseRawSolution`)
- **interface**: `api.tx.electionProviderMultiPhase.submit`
- **summary**:    See [`Pallet::submit`]. 
 
### submitUnsigned(raw_solution: `PalletElectionProviderMultiPhaseRawSolution`, witness: `PalletElectionProviderMultiPhaseSolutionOrSnapshotSize`)
- **interface**: `api.tx.electionProviderMultiPhase.submitUnsigned`
- **summary**:    See [`Pallet::submit_unsigned`]. 

___


## grandpa
 
### noteStalled(delay: `u32`, best_finalized_block_number: `u32`)
- **interface**: `api.tx.grandpa.noteStalled`
- **summary**:    See [`Pallet::note_stalled`]. 
 
### reportEquivocation(equivocation_proof: `SpConsensusGrandpaEquivocationProof`, key_owner_proof: `SpSessionMembershipProof`)
- **interface**: `api.tx.grandpa.reportEquivocation`
- **summary**:    See [`Pallet::report_equivocation`]. 
 
### reportEquivocationUnsigned(equivocation_proof: `SpConsensusGrandpaEquivocationProof`, key_owner_proof: `SpSessionMembershipProof`)
- **interface**: `api.tx.grandpa.reportEquivocationUnsigned`
- **summary**:    See [`Pallet::report_equivocation_unsigned`]. 

___


## imOnline
 
### heartbeat(heartbeat: `PalletImOnlineHeartbeat`, signature: `PalletImOnlineSr25519AppSr25519Signature`)
- **interface**: `api.tx.imOnline.heartbeat`
- **summary**:    See [`Pallet::heartbeat`]. 

___


## palletProxy
 
### removeProxyAccount(proxy_acc: `AccountId32`)
- **interface**: `api.tx.palletProxy.removeProxyAccount`
- **summary**:    See [`Pallet::remove_proxy_account`]. 
 
### setProxyAccount(max_token_usage: `Option<u128>`, token_usage: `u128`, max_task_execution: `Option<u32>`, task_executed: `u32`, proxy: `AccountId32`)
- **interface**: `api.tx.palletProxy.setProxyAccount`
- **summary**:    See [`Pallet::set_proxy_account`]. 
 
### updateProxyAccount(proxy_acc: `AccountId32`, status: `TimePrimitivesProxyStatus`)
- **interface**: `api.tx.palletProxy.updateProxyAccount`
- **summary**:    See [`Pallet::update_proxy_account`]. 

___


## session
 
### purgeKeys()
- **interface**: `api.tx.session.purgeKeys`
- **summary**:    See [`Pallet::purge_keys`]. 
 
### setKeys(keys: `TimechainRuntimeOpaqueSessionKeys`, proof: `Bytes`)
- **interface**: `api.tx.session.setKeys`
- **summary**:    See [`Pallet::set_keys`]. 

___


## staking
 
### bond(value: `Compact<u128>`, payee: `PalletStakingRewardDestination`)
- **interface**: `api.tx.staking.bond`
- **summary**:    See [`Pallet::bond`]. 
 
### bondExtra(max_additional: `Compact<u128>`)
- **interface**: `api.tx.staking.bondExtra`
- **summary**:    See [`Pallet::bond_extra`]. 
 
### cancelDeferredSlash(era: `u32`, slash_indices: `Vec<u32>`)
- **interface**: `api.tx.staking.cancelDeferredSlash`
- **summary**:    See [`Pallet::cancel_deferred_slash`]. 
 
### chill()
- **interface**: `api.tx.staking.chill`
- **summary**:    See [`Pallet::chill`]. 
 
### chillOther(controller: `AccountId32`)
- **interface**: `api.tx.staking.chillOther`
- **summary**:    See [`Pallet::chill_other`]. 
 
### forceApplyMinCommission(validator_stash: `AccountId32`)
- **interface**: `api.tx.staking.forceApplyMinCommission`
- **summary**:    See [`Pallet::force_apply_min_commission`]. 
 
### forceNewEra()
- **interface**: `api.tx.staking.forceNewEra`
- **summary**:    See [`Pallet::force_new_era`]. 
 
### forceNewEraAlways()
- **interface**: `api.tx.staking.forceNewEraAlways`
- **summary**:    See [`Pallet::force_new_era_always`]. 
 
### forceNoEras()
- **interface**: `api.tx.staking.forceNoEras`
- **summary**:    See [`Pallet::force_no_eras`]. 
 
### forceUnstake(stash: `AccountId32`, num_slashing_spans: `u32`)
- **interface**: `api.tx.staking.forceUnstake`
- **summary**:    See [`Pallet::force_unstake`]. 
 
### increaseValidatorCount(additional: `Compact<u32>`)
- **interface**: `api.tx.staking.increaseValidatorCount`
- **summary**:    See [`Pallet::increase_validator_count`]. 
 
### kick(who: `Vec<MultiAddress>`)
- **interface**: `api.tx.staking.kick`
- **summary**:    See [`Pallet::kick`]. 
 
### nominate(targets: `Vec<MultiAddress>`)
- **interface**: `api.tx.staking.nominate`
- **summary**:    See [`Pallet::nominate`]. 
 
### payoutStakers(validator_stash: `AccountId32`, era: `u32`)
- **interface**: `api.tx.staking.payoutStakers`
- **summary**:    See [`Pallet::payout_stakers`]. 
 
### reapStash(stash: `AccountId32`, num_slashing_spans: `u32`)
- **interface**: `api.tx.staking.reapStash`
- **summary**:    See [`Pallet::reap_stash`]. 
 
### rebond(value: `Compact<u128>`)
- **interface**: `api.tx.staking.rebond`
- **summary**:    See [`Pallet::rebond`]. 
 
### scaleValidatorCount(factor: `Percent`)
- **interface**: `api.tx.staking.scaleValidatorCount`
- **summary**:    See [`Pallet::scale_validator_count`]. 
 
### setController()
- **interface**: `api.tx.staking.setController`
- **summary**:    See [`Pallet::set_controller`]. 
 
### setInvulnerables(invulnerables: `Vec<AccountId32>`)
- **interface**: `api.tx.staking.setInvulnerables`
- **summary**:    See [`Pallet::set_invulnerables`]. 
 
### setMinCommission(new: `Perbill`)
- **interface**: `api.tx.staking.setMinCommission`
- **summary**:    See [`Pallet::set_min_commission`]. 
 
### setPayee(payee: `PalletStakingRewardDestination`)
- **interface**: `api.tx.staking.setPayee`
- **summary**:    See [`Pallet::set_payee`]. 
 
### setStakingConfigs(min_nominator_bond: `PalletStakingPalletConfigOpU128`, min_validator_bond: `PalletStakingPalletConfigOpU128`, max_nominator_count: `PalletStakingPalletConfigOpU32`, max_validator_count: `PalletStakingPalletConfigOpU32`, chill_threshold: `PalletStakingPalletConfigOpPercent`, min_commission: `PalletStakingPalletConfigOpPerbill`)
- **interface**: `api.tx.staking.setStakingConfigs`
- **summary**:    See [`Pallet::set_staking_configs`]. 
 
### setValidatorCount(new: `Compact<u32>`)
- **interface**: `api.tx.staking.setValidatorCount`
- **summary**:    See [`Pallet::set_validator_count`]. 
 
### unbond(value: `Compact<u128>`)
- **interface**: `api.tx.staking.unbond`
- **summary**:    See [`Pallet::unbond`]. 
 
### validate(prefs: `PalletStakingValidatorPrefs`)
- **interface**: `api.tx.staking.validate`
- **summary**:    See [`Pallet::validate`]. 
 
### withdrawUnbonded(num_slashing_spans: `u32`)
- **interface**: `api.tx.staking.withdrawUnbonded`
- **summary**:    See [`Pallet::withdraw_unbonded`]. 

___


## sudo
 
### setKey(new: `MultiAddress`)
- **interface**: `api.tx.sudo.setKey`
- **summary**:    See [`Pallet::set_key`]. 
 
### sudo(call: `Call`)
- **interface**: `api.tx.sudo.sudo`
- **summary**:    See [`Pallet::sudo`]. 
 
### sudoAs(who: `MultiAddress`, call: `Call`)
- **interface**: `api.tx.sudo.sudoAs`
- **summary**:    See [`Pallet::sudo_as`]. 
 
### sudoUncheckedWeight(call: `Call`, weight: `SpWeightsWeightV2Weight`)
- **interface**: `api.tx.sudo.sudoUncheckedWeight`
- **summary**:    See [`Pallet::sudo_unchecked_weight`]. 

___


## system
 
### killPrefix(prefix: `Bytes`, subkeys: `u32`)
- **interface**: `api.tx.system.killPrefix`
- **summary**:    See [`Pallet::kill_prefix`]. 
 
### killStorage(keys: `Vec<Bytes>`)
- **interface**: `api.tx.system.killStorage`
- **summary**:    See [`Pallet::kill_storage`]. 
 
### remark(remark: `Bytes`)
- **interface**: `api.tx.system.remark`
- **summary**:    See [`Pallet::remark`]. 
 
### remarkWithEvent(remark: `Bytes`)
- **interface**: `api.tx.system.remarkWithEvent`
- **summary**:    See [`Pallet::remark_with_event`]. 
 
### setCode(code: `Bytes`)
- **interface**: `api.tx.system.setCode`
- **summary**:    See [`Pallet::set_code`]. 
 
### setCodeWithoutChecks(code: `Bytes`)
- **interface**: `api.tx.system.setCodeWithoutChecks`
- **summary**:    See [`Pallet::set_code_without_checks`]. 
 
### setHeapPages(pages: `u64`)
- **interface**: `api.tx.system.setHeapPages`
- **summary**:    See [`Pallet::set_heap_pages`]. 
 
### setStorage(items: `Vec<(Bytes,Bytes)>`)
- **interface**: `api.tx.system.setStorage`
- **summary**:    See [`Pallet::set_storage`]. 

___


## taskMeta
 
### insertCollection(hash: `Text`, task: `Bytes`, validity: `i64`)
- **interface**: `api.tx.taskMeta.insertCollection`
- **summary**:    See [`Pallet::insert_collection`]. 
 
### insertPayableTask(task: `TimePrimitivesAbstractionPayableTask`)
- **interface**: `api.tx.taskMeta.insertPayableTask`
- **summary**:    See [`Pallet::insert_payable_task`]. 
 
### insertTask(task: `TimePrimitivesAbstractionTask`)
- **interface**: `api.tx.taskMeta.insertTask`
- **summary**:    See [`Pallet::insert_task`]. 

___


## taskSchedule
 
### insertPayableTaskSchedule(schedule: `TimePrimitivesAbstractionPayableScheduleInput`)
- **interface**: `api.tx.taskSchedule.insertPayableTaskSchedule`
- **summary**:    See [`Pallet::insert_payable_task_schedule`]. 
 
### insertSchedule(schedule: `TimePrimitivesAbstractionScheduleInput`)
- **interface**: `api.tx.taskSchedule.insertSchedule`
- **summary**:    See [`Pallet::insert_schedule`]. 
 
### updateSchedule(status: `TimePrimitivesAbstractionScheduleStatus`, key: `u64`)
- **interface**: `api.tx.taskSchedule.updateSchedule`
- **summary**:    See [`Pallet::update_schedule`]. 

___


## tesseractSigStorage
 
### forceSetShardOffline(shard_id: `u64`)
- **interface**: `api.tx.tesseractSigStorage.forceSetShardOffline`
- **summary**:    See [`Pallet::force_set_shard_offline`]. 
 
### registerChronicle(member: `AccountId32`)
- **interface**: `api.tx.tesseractSigStorage.registerChronicle`
- **summary**:    See [`Pallet::register_chronicle`]. 
 
### registerShard(members: `Vec<AccountId32>`, collector_index: `Option<u8>`, net: `TimePrimitivesShardingNetwork`)
- **interface**: `api.tx.tesseractSigStorage.registerShard`
- **summary**:    See [`Pallet::register_shard`]. 
 
### reportMisbehavior(shard_id: `u64`, offender: `AccountId32`)
- **interface**: `api.tx.tesseractSigStorage.reportMisbehavior`
- **summary**:    See [`Pallet::report_misbehavior`]. 
 
### storeSignature(auth_sig: `TimePrimitivesCryptoSignature`, signature_data: `[u8;64]`, key_id: `u64`, schedule_cycle: `u64`)
- **interface**: `api.tx.tesseractSigStorage.storeSignature`
- **summary**:    See [`Pallet::store_signature`]. 
 
### submitTssGroupKey(set_id: `u64`, group_key: `[u8;33]`)
- **interface**: `api.tx.tesseractSigStorage.submitTssGroupKey`
- **summary**:    See [`Pallet::submit_tss_group_key`]. 

___


## timestamp
 
### set(now: `Compact<u64>`)
- **interface**: `api.tx.timestamp.set`
- **summary**:    See [`Pallet::set`]. 

___


## treasury
 
### approveProposal(proposal_id: `Compact<u32>`)
- **interface**: `api.tx.treasury.approveProposal`
- **summary**:    See [`Pallet::approve_proposal`]. 
 
### proposeSpend(value: `Compact<u128>`, beneficiary: `MultiAddress`)
- **interface**: `api.tx.treasury.proposeSpend`
- **summary**:    See [`Pallet::propose_spend`]. 
 
### rejectProposal(proposal_id: `Compact<u32>`)
- **interface**: `api.tx.treasury.rejectProposal`
- **summary**:    See [`Pallet::reject_proposal`]. 
 
### removeApproval(proposal_id: `Compact<u32>`)
- **interface**: `api.tx.treasury.removeApproval`
- **summary**:    See [`Pallet::remove_approval`]. 
 
### spend(amount: `Compact<u128>`, beneficiary: `MultiAddress`)
- **interface**: `api.tx.treasury.spend`
- **summary**:    See [`Pallet::spend`]. 

___


## utility
 
### asDerivative(index: `u16`, call: `Call`)
- **interface**: `api.tx.utility.asDerivative`
- **summary**:    See [`Pallet::as_derivative`]. 
 
### batch(calls: `Vec<Call>`)
- **interface**: `api.tx.utility.batch`
- **summary**:    See [`Pallet::batch`]. 
 
### batchAll(calls: `Vec<Call>`)
- **interface**: `api.tx.utility.batchAll`
- **summary**:    See [`Pallet::batch_all`]. 
 
### dispatchAs(as_origin: `TimechainRuntimeOriginCaller`, call: `Call`)
- **interface**: `api.tx.utility.dispatchAs`
- **summary**:    See [`Pallet::dispatch_as`]. 
 
### forceBatch(calls: `Vec<Call>`)
- **interface**: `api.tx.utility.forceBatch`
- **summary**:    See [`Pallet::force_batch`]. 
 
### withWeight(call: `Call`, weight: `SpWeightsWeightV2Weight`)
- **interface**: `api.tx.utility.withWeight`
- **summary**:    See [`Pallet::with_weight`]. 

___


## vesting
 
### claim()
- **interface**: `api.tx.vesting.claim`
- **summary**:    See [`Pallet::claim`]. 
 
### claimFor(dest: `MultiAddress`)
- **interface**: `api.tx.vesting.claimFor`
- **summary**:    See [`Pallet::claim_for`]. 
 
### updateVestingSchedules(who: `MultiAddress`, vesting_schedules: `Vec<AnalogVestingVestingSchedule>`)
- **interface**: `api.tx.vesting.updateVestingSchedules`
- **summary**:    See [`Pallet::update_vesting_schedules`]. 
 
### vestedTransfer(dest: `MultiAddress`, schedule: `AnalogVestingVestingSchedule`)
- **interface**: `api.tx.vesting.vestedTransfer`
- **summary**:    See [`Pallet::vested_transfer`]. 

___


## voterList
 
### putInFrontOf(lighter: `MultiAddress`)
- **interface**: `api.tx.voterList.putInFrontOf`
- **summary**:    See [`Pallet::put_in_front_of`]. 
 
### rebag(dislocated: `MultiAddress`)
- **interface**: `api.tx.voterList.rebag`
- **summary**:    See [`Pallet::rebag`]. 
