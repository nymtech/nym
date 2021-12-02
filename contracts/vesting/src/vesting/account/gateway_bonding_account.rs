use super::BondData;
use crate::errors::ContractError;
use crate::traits::GatewayBondingAccount;
use config::defaults::DEFAULT_MIXNET_CONTRACT_ADDRESS;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract::{ExecuteMsg as MixnetExecuteMsg, Gateway};

use super::Account;

impl GatewayBondingAccount for Account {
    fn try_bond_gateway(
        &self,
        gateway: Gateway,
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

        let bond_data = if let Some(_bond) = self.load_gateway_bond(storage)? {
            return Err(ContractError::AlreadyBonded(
                self.address.as_str().to_string(),
            ));
        } else {
            BondData {
                block_time: env.block.time,
                amount: bond.amount,
            }
        };

        let msg = MixnetExecuteMsg::BondGatewayOnBehalf {
            gateway,
            owner: self.address().into_string(),
        };

        let new_balance = Uint128::new(current_balance.u128() - bond.amount.u128());

        let bond_mixnode_mag = wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![bond])?;

        self.save_balance(new_balance, storage)?;
        self.save_gateway_bond(bond_data, storage)?;

        Ok(Response::new()
            .add_attribute("action", "bond mixnode on behalf")
            .add_message(bond_mixnode_mag))
    }

    fn try_unbond_gateway(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondGatewayOnBehalf {
            owner: self.address().into_string(),
        };

        if let Some(_bond) = self.load_gateway_bond(storage)? {
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

    fn try_track_unbond_gateway(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;

        self.remove_gateway_bond(storage)?;
        Ok(())
    }
}
