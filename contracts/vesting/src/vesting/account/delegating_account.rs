use crate::errors::ContractError;
use crate::traits::DelegatingAccount;
use config::defaults::{DEFAULT_MIXNET_CONTRACT_ADDRESS, DENOM};
use cosmwasm_std::{wasm_execute, Coin, Env, Order, Response, Storage, Timestamp, Uint128};
use mixnet_contract::ExecuteMsg as MixnetExecuteMsg;
use mixnet_contract::IdentityKey;

use super::Account;

impl DelegatingAccount for Account {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        coin: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        let current_balance = self.load_balance(storage)?;

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
        let delegate_to_mixnode =
            wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![coin.clone()])?;
        self.track_delegation(env.block.time, mix_identity, current_balance, coin, storage)?;

        Ok(Response::new()
            .add_attribute("action", "delegate to mixnode on behalf")
            .add_message(delegate_to_mixnode))
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
            DEFAULT_MIXNET_CONTRACT_ADDRESS,
            &msg,
            vec![Coin {
                amount: Uint128::new(0),
                denom: DENOM.to_string(),
            }],
        )?;

        Ok(Response::new()
            .add_attribute("action", "undelegate to mixnode on behalf")
            .add_message(undelegate_from_mixnode))
    }

    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        current_balance: Uint128,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let delegation_key = (mix_identity.as_bytes(), block_time.seconds());

        let new_delegation = if let Some(existing_delegation) =
            self.delegations().may_load(storage, delegation_key)?
        {
            existing_delegation + delegation.amount
        } else {
            delegation.amount
        };

        self.delegations()
            .save(storage, delegation_key, &new_delegation)?;

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
        let mix_bytes = mix_identity.as_bytes();

        // Iterate over keys matching the prefix and remove them from the map
        let block_times = self
            .delegations()
            .prefix_de(mix_bytes)
            .keys_de(storage, None, None, Order::Ascending)
            // Scan will blow up on first error
            .scan((), |_, x| x.ok())
            .collect::<Vec<u64>>();

        for t in block_times {
            self.delegations().remove(storage, (mix_bytes, t))
        }

        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());

        self.save_balance(new_balance, storage)?;

        Ok(())
    }
}
