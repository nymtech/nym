use super::PledgeData;
use crate::errors::ContractError;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::traits::MixnodeBondingAccount;
use config::defaults::DENOM;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::{ExecuteMsg as MixnetExecuteMsg, MixNode};
use vesting_contract_common::events::{
    new_vesting_mixnode_bonding_event, new_vesting_mixnode_unbonding_event,
};

use super::Account;

impl MixnodeBondingAccount for Account {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        owner_signature: String,
        pledge: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let current_balance = self.load_balance(storage)?;

        if current_balance < pledge.amount {
            return Err(ContractError::InsufficientBalance(
                self.owner_address().as_str().to_string(),
                current_balance.u128(),
            ));
        }

        let pledge_data = if self.load_mixnode_pledge(storage)?.is_some() {
            return Err(ContractError::AlreadyBonded(
                self.owner_address().as_str().to_string(),
            ));
        } else {
            PledgeData {
                block_time: env.block.time,
                amount: pledge.amount,
            }
        };

        let msg = MixnetExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
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

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondMixnodeOnBehalf {
            owner: self.owner_address().into_string(),
        };

        if self.load_mixnode_pledge(storage)?.is_some() {
            let unbond_msg = wasm_execute(
                MIXNET_CONTRACT_ADDRESS.load(storage)?,
                &msg,
                vec![Coin::new(0, DENOM)],
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
