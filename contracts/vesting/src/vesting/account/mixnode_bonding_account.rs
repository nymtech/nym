// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::Account;
use crate::errors::ContractError;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::traits::MixnodeBondingAccount;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::mixnode::MixNodeConfigUpdate;
use mixnet_contract_common::mixnode::MixNodeCostParams;
use mixnet_contract_common::{ExecuteMsg as MixnetExecuteMsg, MixNode};
use vesting_contract_common::events::{
    new_vesting_mixnode_bonding_event, new_vesting_mixnode_unbonding_event,
    new_vesting_pledge_more_event, new_vesting_update_mixnode_config_event,
    new_vesting_update_mixnode_cost_params_event,
};
use vesting_contract_common::PledgeData;

impl MixnodeBondingAccount for Account {
    fn try_claim_operator_reward(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::WithdrawOperatorRewardOnBehalf {
            owner: self.owner_address().into_string(),
        };

        let compound_operator_reward_msg =
            wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(compound_operator_reward_msg))
    }

    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: String,
        pledge: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let current_balance = self.ensure_valid_additional_stake(&pledge, storage)?;

        let pledge_data = if self.load_mixnode_pledge(storage)?.is_some() {
            return Err(ContractError::AlreadyBonded(
                self.owner_address().as_str().to_string(),
            ));
        } else {
            PledgeData::new(pledge.clone(), env.block.time)
        };

        let msg = MixnetExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
            cost_params,
            owner: self.owner_address().into_string(),
            owner_signature,
        };

        let new_balance = Uint128::new(current_balance.u128() - pledge.amount.u128());

        let bond_mixnode_mag =
            wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![pledge])?;

        self.save_balance(new_balance, storage)?;
        self.save_mixnode_pledge(pledge_data, storage)?;

        Ok(Response::new()
            .add_message(bond_mixnode_mag)
            .add_event(new_vesting_mixnode_bonding_event()))
    }

    fn try_pledge_additional_tokens(
        &self,
        additional_pledge: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let current_balance = self.ensure_valid_additional_stake(&additional_pledge, storage)?;

        let mut pledge_data = if let Some(pledge_data) = self.load_mixnode_pledge(storage)? {
            pledge_data
        } else {
            return Err(ContractError::NoBondFound(
                self.owner_address().as_str().to_string(),
            ));
        };

        // need a second pair of eyes here to make sure updating existing timestamp on pledge data
        // is not going to have some unexpected consequences
        pledge_data.amount.amount += additional_pledge.amount;
        pledge_data.block_time = env.block.time;

        let msg = MixnetExecuteMsg::PledgeMoreOnBehalf {
            owner: self.owner_address().into_string(),
        };

        let new_balance = Uint128::new(current_balance.u128() - additional_pledge.amount.u128());

        let pledge_more_mag = wasm_execute(
            MIXNET_CONTRACT_ADDRESS.load(storage)?,
            &msg,
            vec![additional_pledge],
        )?;

        self.save_balance(new_balance, storage)?;
        self.save_mixnode_pledge(pledge_data, storage)?;

        Ok(Response::new()
            .add_message(pledge_more_mag)
            .add_event(new_vesting_pledge_more_event()))
    }

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondMixnodeOnBehalf {
            owner: self.owner_address().into_string(),
        };

        if self.load_mixnode_pledge(storage)?.is_some() {
            let unbond_msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

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

    fn try_update_mixnode_config(
        &self,
        new_config: MixNodeConfigUpdate,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UpdateMixnodeConfigOnBehalf {
            new_config,
            owner: self.owner_address().into_string(),
        };

        let update_mixnode_config_msg =
            wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new()
            .add_message(update_mixnode_config_msg)
            .add_event(new_vesting_update_mixnode_config_event()))
    }

    fn try_update_mixnode_cost_params(
        &self,
        new_costs: MixNodeCostParams,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UpdateMixnodeCostParamsOnBehalf {
            new_costs,
            owner: self.owner_address().into_string(),
        };

        let update_mixnode_costs_msg =
            wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new()
            .add_message(update_mixnode_costs_msg)
            .add_event(new_vesting_update_mixnode_cost_params_event()))
    }
}
