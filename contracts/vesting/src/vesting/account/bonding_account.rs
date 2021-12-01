use super::BondData;
use crate::errors::ContractError;
use crate::traits::BondingAccount;
use config::defaults::DEFAULT_MIXNET_CONTRACT_ADDRESS;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract::{ExecuteMsg as MixnetExecuteMsg, MixNode};

use super::Account;

impl BondingAccount for Account {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        owner_signature: String,
        bond: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let current_balance = self.load_balance(storage)?;

        if current_balance < bond.amount {
            return Err(ContractError::InsufficientBalance(
                self.address.as_str().to_string(),
                current_balance.u128(),
            ));
        }

        let bond_data = if let Some(_bond) = self.load_bond(storage)? {
            return Err(ContractError::AlreadyBonded(
                self.address.as_str().to_string(),
            ));
        } else {
            BondData {
                block_time: env.block.time,
                amount: bond.amount,
            }
        };

        let msg = MixnetExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
            owner: self.address().into_string(),
            owner_signature,
        };

        let new_balance = Uint128::new(current_balance.u128() - bond.amount.u128());

        let bond_mixnode_mag = wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![bond])?;

        self.save_balance(new_balance, storage)?;
        self.save_bond(bond_data, storage)?;

        Ok(Response::new()
            .add_attribute("action", "bond mixnode on behalf")
            .add_message(bond_mixnode_mag))
    }

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondMixnodeOnBehalf {
            owner: self.address().into_string(),
        };

        if let Some(_bond) = self.load_bond(storage)? {
            let unbond_msg = wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![])?;

            Ok(Response::new()
                .add_attribute("action", "unbond mixnode on behalf")
                .add_message(unbond_msg))
        } else {
            Err(ContractError::NoBondFound(
                self.address.as_str().to_string(),
            ))
        }
    }

    fn track_unbond(&self, amount: Coin, storage: &mut dyn Storage) -> Result<(), ContractError> {
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;

        self.remove_bond(storage)?;
        Ok(())
    }
}
