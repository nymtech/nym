use super::PledgeData;
use crate::storage::MIXNET_CONTRACT_ADDRESS;
use crate::traits::GatewayBondingAccount;
use crate::vesting::account::StorableVestingAccountExt;
use contracts_common::signing::MessageSignature;
use cosmwasm_std::{wasm_execute, Coin, Env, Response, Storage, Uint128};
use mixnet_contract_common::{
    gateway::GatewayConfigUpdate, ExecuteMsg as MixnetExecuteMsg, Gateway,
};
use vesting_contract_common::events::{
    new_vesting_gateway_bonding_event, new_vesting_gateway_unbonding_event,
    new_vesting_update_gateway_config_event,
};
use vesting_contract_common::VestingContractError;

use super::Account;

impl GatewayBondingAccount for Account {
    fn try_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: MessageSignature,
        pledge: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, VestingContractError> {
        let current_balance = self.ensure_valid_additional_stake(&pledge, storage)?;

        let pledge_data = if self.load_gateway_pledge(storage)?.is_some() {
            return Err(VestingContractError::AlreadyBonded(
                self.owner_address().as_str().to_string(),
            ));
        } else {
            PledgeData::new(pledge.clone(), env.block.time)
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

    fn try_unbond_gateway(&self, storage: &dyn Storage) -> Result<Response, VestingContractError> {
        let msg = MixnetExecuteMsg::UnbondGatewayOnBehalf {
            owner: self.owner_address().into_string(),
        };

        if let Some(_bond) = self.load_gateway_pledge(storage)? {
            let unbond_msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

            Ok(Response::new()
                .add_message(unbond_msg)
                .add_event(new_vesting_gateway_unbonding_event()))
        } else {
            Err(VestingContractError::NoBondFound(
                self.owner_address().as_str().to_string(),
            ))
        }
    }

    fn try_track_unbond_gateway(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;

        self.remove_gateway_pledge(storage)?;
        Ok(())
    }

    fn try_update_gateway_config(
        &self,
        new_config: GatewayConfigUpdate,
        storage: &mut dyn Storage,
    ) -> Result<Response, VestingContractError> {
        let msg = MixnetExecuteMsg::UpdateGatewayConfigOnBehalf {
            new_config,
            owner: self.owner_address().into_string(),
        };

        let update_gateway_config_msg =
            wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new()
            .add_message(update_gateway_config_msg)
            .add_event(new_vesting_update_gateway_config_event()))
    }
}
