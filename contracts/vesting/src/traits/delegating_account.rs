use cosmwasm_std::{Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::NodeId;
use vesting_contract_common::VestingContractError;

pub trait DelegatingAccount {
    fn try_claim_delegator_reward(
        &self,
        mix_id: NodeId,
        storage: &dyn Storage,
    ) -> Result<Response, VestingContractError>;

    fn try_delegate_to_mixnode(
        &self,
        mix_id: NodeId,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, VestingContractError>;

    fn try_undelegate_from_mixnode(
        &self,
        mix_id: NodeId,
        storage: &dyn Storage,
    ) -> Result<Response, VestingContractError>;

    // track_delegation performs internal vesting accounting necessary when
    // delegating from a vesting account. It accepts the current block height, the
    // delegation amount and balance of all coins whose denomination exists in
    // the account's original vesting balance.
    fn track_delegation(
        &self,
        block_height: u64,
        mix_id: NodeId,
        // Save some gas by passing it in
        current_balance: Uint128,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;
    // track_undelegation performs internal vesting accounting necessary when a
    // vesting account performs an undelegation.
    fn track_undelegation(
        &self,
        mix_id: NodeId,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;
    fn track_migrated_delegation(
        &self,
        mix_id: NodeId,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;
}
