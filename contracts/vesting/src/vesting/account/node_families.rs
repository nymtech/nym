use super::Account;
use crate::{storage::MIXNET_CONTRACT_ADDRESS, traits::NodeFamilies};
use contracts_common::signing::MessageSignature;
use cosmwasm_std::{wasm_execute, Response, Storage};
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::{ExecuteMsg as MixnetExecuteMsg, IdentityKeyRef};
use vesting_contract_common::VestingContractError;

impl NodeFamilies for Account {
    fn try_create_family(
        &self,
        storage: &dyn Storage,
        label: String,
    ) -> Result<Response, VestingContractError> {
        let msg = MixnetExecuteMsg::CreateFamilyOnBehalf {
            owner_address: self.owner_address().into_string(),
            label,
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }

    fn try_join_family(
        &self,
        storage: &dyn Storage,
        join_permit: MessageSignature,
        family_head: FamilyHead,
    ) -> Result<Response, VestingContractError> {
        let msg = MixnetExecuteMsg::JoinFamilyOnBehalf {
            member_address: self.owner_address().to_string(),
            join_permit,
            family_head,
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }

    fn try_leave_family(
        &self,
        storage: &dyn Storage,
        family_head: FamilyHead,
    ) -> Result<Response, VestingContractError> {
        let msg = MixnetExecuteMsg::LeaveFamilyOnBehalf {
            member_address: self.owner_address().to_string(),
            family_head,
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }

    fn try_head_kick_member(
        &self,
        storage: &dyn Storage,
        member: IdentityKeyRef<'_>,
    ) -> Result<Response, VestingContractError> {
        let msg = MixnetExecuteMsg::KickFamilyMemberOnBehalf {
            head_address: self.owner_address().to_string(),
            member: member.to_string(),
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }
}
