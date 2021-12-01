use crate::errors::ContractError;
use cosmwasm_std::{Coin, Env, Response, Storage};
use mixnet_contract::MixNode;

pub trait BondingAccount {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        owner_signature: String,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError>;

    fn track_unbond(&self, amount: Coin, storage: &mut dyn Storage) -> Result<(), ContractError>;
}
