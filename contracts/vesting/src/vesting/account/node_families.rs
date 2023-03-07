use super::Account;
use crate::{errors::ContractError, storage::MIXNET_CONTRACT_ADDRESS, traits::NodeFamilies};
use contracts_common::signing::MessageSignature;
use cosmwasm_std::{wasm_execute, Response, Storage};
use mixnet_contract_common::{ExecuteMsg as MixnetExecuteMsg, IdentityKeyRef};

impl NodeFamilies for Account {
    fn try_create_family(
        &self,
        storage: &dyn Storage,
        owner_signature: MessageSignature,
        label: String,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::CreateFamilyOnBehalf {
            owner_address: self.owner_address().to_string(),
            owner_signature,
            label,
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }

    fn try_join_family(
        &self,
        storage: &dyn Storage,
        join_permit: MessageSignature,
        family_head: IdentityKeyRef,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::JoinFamilyOnBehalf {
            member_address: self.owner_address().to_string(),
            join_permit,
            family_head: family_head.to_string(),
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }

    fn try_leave_family(
        &self,
        storage: &dyn Storage,
        node_identity_signature: String,
        family_head: IdentityKeyRef,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::LeaveFamilyOnBehalf {
            member_address: self.owner_address().to_string(),
            node_identity_signature,
            family_head: family_head.to_string(),
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }

    fn try_head_kick_member(
        &self,
        storage: &dyn Storage,
        signature: String,
        member: IdentityKeyRef<'_>,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::KickFamilyMemberOnBehalf {
            head_address: self.owner_address().to_string(),
            signature,
            member: member.to_string(),
        };

        let msg = wasm_execute(MIXNET_CONTRACT_ADDRESS.load(storage)?, &msg, vec![])?;

        Ok(Response::new().add_message(msg))
    }
}
