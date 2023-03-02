use crate::errors::ContractError;
use contracts_common::signing::MessageSignature;
use cosmwasm_std::{Response, Storage};
use mixnet_contract_common::IdentityKeyRef;

pub trait NodeFamilies {
    fn try_create_family(
        &self,
        storage: &dyn Storage,
        owner_signature: MessageSignature,
        label: String,
    ) -> Result<Response, ContractError>;

    fn try_join_family(
        &self,
        storage: &dyn Storage,
        node_identity_signature: String,
        family_signature: String,
        family_head: IdentityKeyRef,
    ) -> Result<Response, ContractError>;

    fn try_leave_family(
        &self,
        storage: &dyn Storage,
        signature: String,
        family_head: IdentityKeyRef,
    ) -> Result<Response, ContractError>;

    fn try_head_kick_member(
        &self,
        storage: &dyn Storage,
        signature: String,
        member: IdentityKeyRef<'_>,
    ) -> Result<Response, ContractError>;
}
