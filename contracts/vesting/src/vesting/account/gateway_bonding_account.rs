use super::PledgeData;
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

        let pledge_data = if self.load_gateway_pledge(storage)?.is_some() {
            return Err(ContractError::AlreadyBonded(
                self.owner_address().as_str().to_string(),
            ));
        } else {
            PledgeData {
                block_time: env.block.time,
                amount: pledge.amount,
            }
        };

        let msg = MixnetExecuteMsg::BondGatewayOnBehalf {
            gateway,
            owner: self.owner_address().into_string(),
            owner_signature,
        };

        let new_balance = Uint128::new(current_balance.u128() - pledge.amount.u128());

        let bond_gateway_msg = wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![pledge])?;

        self.save_balance(new_balance, storage)?;
        self.save_gateway_pledge(pledge_data, storage)?;

        Ok(Response::new()
            .add_attribute("action", "bond gateway on behalf")
            .add_message(bond_gateway_msg))
    }

    fn try_unbond_gateway(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondGatewayOnBehalf {
            owner: self.owner_address().into_string(),
        };

        if let Some(_bond) = self.load_gateway_pledge(storage)? {
            let unbond_msg = wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![])?;

            Ok(Response::new()
                .add_attribute("action", "unbond gateway on behalf")
                .add_message(unbond_msg))
        } else {
            Err(ContractError::NoBondFound(
                self.owner_address().as_str().to_string(),
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
