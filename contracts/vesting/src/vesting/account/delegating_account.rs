use crate::errors::ContractError;
use crate::storage::locked_pledge_cap;
use crate::storage::save_delegation;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::storage::MIX_DENOM;
use crate::traits::DelegatingAccount;
use crate::traits::VestingAccount;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::ExecuteMsg as MixnetExecuteMsg;
use mixnet_contract_common::IdentityKey;
use vesting_contract_common::events::{
    new_vesting_delegation_event, new_vesting_undelegation_event,
};
use vesting_contract_common::one_ucoin;

use super::Account;

impl DelegatingAccount for Account {
    fn try_claim_delegator_reward(
        &self,
        mix_identity: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::ClaimDelegatorRewardOnBehalf {
            owner: self.owner_address().to_string(),
            mix_identity,
        };

        let compound_delegator_reward_msg = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![one_ucoin(MIX_DENOM.load(storage)?)],
        )?;

        Ok(Response::new().add_message(compound_delegator_reward_msg))
    }

    fn try_compound_delegator_reward(
        &self,
        mix_identity: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError> {
        todo!()
        // let msg = MixnetExecuteMsg::CompoundDelegatorRewardOnBehalf {
        //     owner: self.owner_address().to_string(),
        //     mix_identity,
        // };
        // let compound_delegator_reward_msg = wasm_execute(
        //     MIXNET_CONTRACT_ADDRESS.load(storage)?,
        //     &msg,
        //     vec![one_ucoin(MIX_DENOM.load(storage)?)],
        // )?;
        //
        // Ok(Response::new().add_message(compound_delegator_reward_msg))
    }

    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
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
            mix_identity: mix_identity.clone(),
            delegate: self.owner_address().into_string(),
        };
        let delegate_to_mixnode = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![coin.clone()],
        )?;
        self.track_delegation(
            env.block.height,
            mix_identity,
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
        mix_identity: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError> {
        if !self.any_delegation_for_mix(&mix_identity, storage) {
            return Err(ContractError::NoSuchDelegation(
                self.owner_address(),
                mix_identity,
            ));
        }

        let msg = MixnetExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_identity,
            delegate: self.owner_address().into_string(),
        };
        let undelegate_from_mixnode = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![one_ucoin(MIX_DENOM.load(storage)?)],
        )?;

        Ok(Response::new()
            .add_message(undelegate_from_mixnode)
            .add_event(new_vesting_undelegation_event()))
    }

    fn track_delegation(
        &self,
        block_height: u64,
        mix_identity: IdentityKey,
        current_balance: Uint128,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        save_delegation(
            (self.storage_key(), mix_identity, block_height),
            delegation.amount,
            storage,
        )?;
        let new_balance = Uint128::new(current_balance.u128() - delegation.amount.u128());
        self.save_balance(new_balance, storage)?;
        Ok(())
    }

    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        self.remove_delegations_for_mix(&mix_identity, storage)?;
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;
        Ok(())
    }
}
