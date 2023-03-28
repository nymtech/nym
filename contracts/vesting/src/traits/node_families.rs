use crate::errors::ContractError;
use contracts_common::signing::MessageSignature;
use cosmwasm_std::{Response, Storage};
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::IdentityKeyRef;

pub trait NodeFamilies {
    fn try_create_family(
        &self,
        storage: &dyn Storage,
        label: String,
    ) -> Result<Response, ContractError>;

    fn try_join_family(
        &self,
        storage: &dyn Storage,
        join_permit: MessageSignature,
        family_head: FamilyHead,
    ) -> Result<Response, ContractError>;

    fn try_leave_family(
        &self,
        storage: &dyn Storage,
        family_head: FamilyHead,
    ) -> Result<Response, ContractError>;

    fn try_head_kick_member(
        &self,
        storage: &dyn Storage,
        member: IdentityKeyRef<'_>,
    ) -> Result<Response, ContractError>;
}
