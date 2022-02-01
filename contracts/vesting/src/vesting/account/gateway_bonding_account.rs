use super::PledgeData;
use crate::errors::ContractError;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::traits::GatewayBondingAccount;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::{ExecuteMsg as MixnetExecuteMsg, Gateway};
use vesting_contract_common::events::{
    new_vesting_gateway_bonding_event, new_vesting_gateway_unbonding_event,
};
use vesting_contract_common::one_unym;

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

        let bond_gateway_msg =
            wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![pledge])?;

        self.save_balance(new_balance, storage)?;
        self.save_gateway_pledge(pledge_data, storage)?;

        Ok(Response::new()
            .add_message(bond_gateway_msg)
            .add_event(new_vesting_gateway_bonding_event()))
    }

    fn try_unbond_gateway(&self, storage: &dyn Storage) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondGatewayOnBehalf {
            owner: self.owner_address().into_string(),
        };

        if let Some(_bond) = self.load_gateway_pledge(storage)? {
            let unbond_msg = wasm_execute(
                MIXNET_CONTRACT_ADDRESS.load(storage)?,
                &msg,
                vec![one_unym()],
            )?;

            Ok(Response::new()
                .add_message(unbond_msg)
                .add_event(new_vesting_gateway_unbonding_event()))
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

        self.remove_gateway_pledge(storage)?;
        Ok(())
    }
}
