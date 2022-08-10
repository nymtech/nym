use crate::errors::ContractError;
use cosmwasm_std::{Coin, Env, Response, Storage};
use mixnet_contract_common::{
    mixnode::{MixNodeConfigUpdate, MixNodeCostParams},
    Gateway, MixNode,
};

pub trait MixnodeBondingAccount {
    fn try_claim_operator_reward(&self, storage: &dyn Storage) -> Result<Response, ContractError>;

    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: String,
        pledge: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_unbond_mixnode(&self, storage: &dyn Storage) -> Result<Response, ContractError>;

    fn try_track_unbond_mixnode(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;

    fn try_update_mixnode_config(
        &self,
        new_config: MixNodeConfigUpdate,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;
}

pub trait GatewayBondingAccount {
    fn try_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: String,
        pledge: Coin,
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
