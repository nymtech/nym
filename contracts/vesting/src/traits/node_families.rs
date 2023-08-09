use contracts_common::signing::MessageSignature;
use cosmwasm_std::{Response, Storage};
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::IdentityKeyRef;
use vesting_contract_common::VestingContractError;

pub trait NodeFamilies {
    fn try_create_family(
        &self,
        storage: &dyn Storage,
        label: String,
    ) -> Result<Response, VestingContractError>;

    fn try_join_family(
        &self,
        storage: &dyn Storage,
        join_permit: MessageSignature,
        family_head: FamilyHead,
    ) -> Result<Response, VestingContractError>;

    fn try_leave_family(
        &self,
        storage: &dyn Storage,
        family_head: FamilyHead,
    ) -> Result<Response, VestingContractError>;

    fn try_head_kick_member(
        &self,
        storage: &dyn Storage,
        member: IdentityKeyRef<'_>,
    ) -> Result<Response, VestingContractError>;
}
