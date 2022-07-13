use crate::errors::ContractError;
use crate::storage::locked_pledge_cap;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::storage::MIX_DENOM;
use crate::traits::MixnodeBondingAccount;
use crate::traits::VestingAccount;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::{ExecuteMsg as MixnetExecuteMsg, MixNode};
use vesting_contract_common::events::{
    new_vesting_mixnode_bonding_event, new_vesting_mixnode_unbonding_event,
    new_vesting_update_mixnode_config_event,
};

use vesting_contract_common::one_ucoin;
use vesting_contract_common::PledgeData;

use super::Account;

impl MixnodeBondingAccount for Account {
    fn try_claim_operator_reward(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        todo!()
        // let msg = MixnetExecuteMsg::ClaimOperatorRewardOnBehalf {
        //     owner: self.owner_address().into_string(),
        // };
        //
        // let compound_operator_reward_msg = wasm_execute(
        //     MIXNET_CONTRACT_ADDRESS.load(storage)?,
        //     &msg,
        //     vec![one_ucoin(MIX_DENOM.load(storage)?)],
        // )?;
        //
        // Ok(Response::new().add_message(compound_operator_reward_msg))
    }

    fn try_compound_operator_reward(
        &self,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError> {
        todo!()
        // let msg = MixnetExecuteMsg::CompoundOperatorRewardOnBehalf {
        //     owner: self.owner_address().into_string(),
        // };
        //
        // let compound_operator_reward_msg = wasm_execute(
        //     MIXNET_CONTRACT_ADDRESS.load(storage)?,
        //     &msg,
        //     vec![one_ucoin(MIX_DENOM.load(storage)?)],
        // )?;
        //
        // Ok(Response::new().add_message(compound_operator_reward_msg))
    }

    fn try_update_mixnode_config(
        &self,
        profit_margin_percent: u8,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        todo!()
        // let msg = MixnetExecuteMsg::UpdateMixnodeConfigOnBehalf {
        //     profit_margin_percent,
        //     owner: self.owner_address().into_string(),
        // };
        //
        // let update_mixnode_config_msg = wasm_execute(
        //     MIXNET_CONTRACT_ADDRESS.load(storage)?,
        //     &msg,
        //     vec![one_ucoin(MIX_DENOM.load(storage)?)],
        // )?;
        //
        // Ok(Response::new()
        //     .add_message(update_mixnode_config_msg)
        //     .add_event(new_vesting_update_mixnode_config_event()))
    }

    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        owner_signature: String,
        pledge: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        todo!()
        // let current_balance = self.load_balance(storage)?;
        // let total_pledged_after = self.total_pledged_locked(storage, env)? + pledge.amount;
        // let locked_pledge_cap = locked_pledge_cap(storage);
        //
        // if locked_pledge_cap < total_pledged_after {
        //     return Err(ContractError::LockedPledgeCapReached {
        //         current: total_pledged_after,
        //         cap: locked_pledge_cap,
        //     });
        // }
        //
        // if current_balance < pledge.amount {
        //     return Err(ContractError::InsufficientBalance(
        //         self.owner_address().as_str().to_string(),
        //         current_balance.u128(),
        //     ));
        // }
        //
        // let pledge_data = if self.load_mixnode_pledge(storage)?.is_some() {
        //     return Err(ContractError::AlreadyBonded(
        //         self.owner_address().as_str().to_string(),
        //     ));
        // } else {
        //     PledgeData::new(pledge.clone(), env.block.time)
        // };
        //
        // let msg = MixnetExecuteMsg::BondMixnodeOnBehalf {
        //     mix_node,
        //     owner: self.owner_address().into_string(),
        //     owner_signature,
        // };
        //
        // let new_balance = Uint128::new(current_balance.u128() - pledge.amount.u128());
        //
        // let bond_mixnode_mag =
        //     wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![pledge])?;
        //
        // self.save_balance(new_balance, storage)?;
        // self.save_mixnode_pledge(pledge_data, storage)?;
        //
        // Ok(Response::new()
        //     .add_message(bond_mixnode_mag)
        //     .add_event(new_vesting_mixnode_bonding_event()))
    }

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondMixnodeOnBehalf {
            owner: self.owner_address().into_string(),
        };

        if self.load_mixnode_pledge(storage)?.is_some() {
            let unbond_msg = wasm_execute(
                MIXNET_CONTRACT_ADDRESS.load(storage)?,
                &msg,
                vec![one_ucoin(MIX_DENOM.load(storage)?)],
            )?;

            Ok(Response::new()
                .add_message(unbond_msg)
                .add_event(new_vesting_mixnode_unbonding_event()))
        } else {
            Err(ContractError::NoBondFound(
                self.owner_address().as_str().to_string(),
            ))
        }
    }

    fn try_track_unbond_mixnode(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;

        self.remove_mixnode_pledge(storage)?;
        Ok(())
    }
}
