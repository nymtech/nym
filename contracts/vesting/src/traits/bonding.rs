use crate::errors::ContractError;
use cosmwasm_std::{Coin, Env, Response, Storage};
use mixnet_contract::{Gateway, MixNode};

pub trait MixnodeBondingAccount {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError>;

    fn try_track_unbond_mixnode(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
}

pub trait GatewayBondingAccount {
    fn try_bond_gateway(
        &self,
        gateway: Gateway,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_unbond_gateway(&self, storage: &dyn Storage) -> Result<Response, ContractError>;

    fn try_track_unbond_gateway(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
}
