use crate::errors::ContractError;
use crate::storage::locked_pledge_cap;
use crate::storage::save_delegation;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::traits::DelegatingAccount;
use crate::traits::VestingAccount;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::ExecuteMsg as MixnetExecuteMsg;
use mixnet_contract_common::NodeId;
use vesting_contract_common::events::{
    new_vesting_delegation_event, new_vesting_undelegation_event,
};

use super::Account;

impl DelegatingAccount for Account {
    fn try_claim_delegator_reward(
        &self,
        mix_id: NodeId,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::WithdrawDelegatorRewardOnBehalf {
            owner: self.owner_address().to_string(),
            mix_id,
        };
        
        let compound_delegator_reward_msg = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![],
        )?;
        
        Ok(Response::new().add_message(compound_delegator_reward_msg))
    }

    fn try_delegate_to_mixnode(
        &self,
        mix_id: NodeId,
        coin: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let current_balance = self.load_balance(storage)?;
        let total_pledged_after = self.total_pledged_locked(storage, env)? + coin.amount;
        let locked_pledge_cap = locked_pledge_cap(storage);

        if locked_pledge_cap < total_pledged_after {
            return Err(ContractError::LockedPledgeCapReached {
                current: total_pledged_after,
                cap: locked_pledge_cap,
            });
        }

        if current_balance < coin.amount {
            return Err(ContractError::InsufficientBalance(
                self.owner_address().as_str().to_string(),
                current_balance.u128(),
            ));
        }

        let msg = MixnetExecuteMsg::DelegateToMixnodeOnBehalf {
            mix_id,
            delegate: self.owner_address().into_string(),
        };
        let delegate_to_mixnode = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![coin.clone()],
        )?;
        self.track_delegation(
            env.block.height,
            mix_id,
            current_balance,
            coin,
            storage,
        )?;

        Ok(Response::new()
            .add_message(delegate_to_mixnode)
            .add_event(new_vesting_delegation_event()))
    }

    fn try_undelegate_from_mixnode(
        &self,
        mix_id: NodeId,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError> {
        if !self.any_delegation_for_mix(mix_id, storage) {
            return Err(ContractError::NoSuchDelegation(
                self.owner_address(),
                mix_id,
            ));
        }
        
        let msg = MixnetExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_id,
            delegate: self.owner_address().into_string(),
        };
        let undelegate_from_mixnode = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![],
        )?;
        
        Ok(Response::new()
            .add_message(undelegate_from_mixnode)
            .add_event(new_vesting_undelegation_event()))
    }

    fn track_delegation(
        &self,
        block_height: u64,
        mix_id: NodeId,
        current_balance: Uint128,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        save_delegation(
            (self.storage_key(), mix_id, block_height),
            delegation.amount,
            storage,
        )?;
        let new_balance = Uint128::new(current_balance.u128() - delegation.amount.u128());
        self.save_balance(new_balance, storage)?;
        Ok(())
    }

    fn track_undelegation(
        &self,
        mix_id: NodeId,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        self.remove_delegations_for_mix(mix_id, storage)?;
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;
        Ok(())
    }
}
