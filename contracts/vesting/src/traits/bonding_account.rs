use crate::errors::ContractError;
use cosmwasm_std::{Coin, Env, Response, Storage, Timestamp};
use mixnet_contract::MixNode;

pub trait BondingAccount {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_unbond_mixnode(&self) -> Result<Response, ContractError>;

    fn track_bond(
        &self,
        block_time: Timestamp,
        bond: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;

    fn track_unbond(&self, amount: Coin, storage: &mut dyn Storage) -> Result<(), ContractError>;
}
