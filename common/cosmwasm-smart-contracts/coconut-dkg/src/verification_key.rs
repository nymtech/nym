// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::msg::ExecuteMsg;
use crate::types::{EpochId, NodeIndex};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, to_json_binary, Addr, CosmosMsg, StdResult, Timestamp, WasmMsg};
use cw_utils::Expiration;
use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;

pub type VerificationKeyShare = String;

#[cw_serde]
pub struct ContractVKShare {
    pub share: VerificationKeyShare,
    pub announce_address: String,
    pub node_index: NodeIndex,
    pub owner: Addr,
    pub epoch_id: EpochId,
    pub verified: bool,
}

#[cw_serde]
pub struct VkShareResponse {
    pub owner: Addr,
    pub epoch_id: EpochId,
    pub share: Option<ContractVKShare>,
}

#[cw_serde]
pub struct PagedVKSharesResponse {
    pub shares: Vec<ContractVKShare>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<Addr>,
}

pub fn to_cosmos_msg(
    owner: Addr,
    resharing: bool,
    epoch_id: EpochId,
    coconut_dkg_addr: String,
    multisig_addr: String,
    expiration_time: Timestamp,
) -> StdResult<CosmosMsg> {
    let verify_vk_share_req = ExecuteMsg::VerifyVerificationKeyShare {
        owner: owner.to_string(),
        resharing,
        epoch_id,
    };
    let verify_vk_share_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: coconut_dkg_addr,
        msg: to_json_binary(&verify_vk_share_req)?,
        funds: vec![],
    });
    let req = MultisigExecuteMsg::Propose {
        title: String::from("Verify VK share, as ordered by Coconut DKG Contract"),
        description: String::new(),
        msgs: vec![verify_vk_share_msg],
        latest: Some(Expiration::AtTime(expiration_time)),
    };
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: multisig_addr,
        msg: to_json_binary(&req)?,
        funds: vec![],
    });

    Ok(msg)
}

// DKG SAFETY:
// each legit verification proposal will only contain a single execute msg,
// if they have more than one, we can safely ignore it
pub fn owner_from_cosmos_msgs(msgs: &[CosmosMsg]) -> Option<String> {
    if msgs.len() != 1 {
        return None;
    }

    if let Some(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: _,
        msg,
        funds: _,
    })) = msgs.first()
    {
        if let Ok(ExecuteMsg::VerifyVerificationKeyShare { owner, .. }) =
            from_json::<ExecuteMsg>(msg)
        {
            return Some(owner);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Binary;

    /// Readers built before a field was added to a verification order must still be able to
    /// read one, since an api can be older than the contract it is talking to and
    /// `owner_from_cosmos_msgs` is how it finds the proposals it has to vote on.
    ///
    /// `cw_serde` does not ask for `deny_unknown_fields`, so serde ignores what it does not
    /// recognise. That is the property this relies on, and it is not visible at the call site.
    #[test]
    fn a_verification_order_carrying_an_unknown_field_is_still_readable() {
        // written out as bytes rather than built from the current type, so that it keeps
        // standing in for an order this version has never seen
        let raw = br#"{"verify_verification_key_share":{
            "owner":"n1alice",
            "resharing":false,
            "epoch_id":42,
            "something_a_later_version_added":"ignore me"
        }}"#;

        let msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "n1dkg".to_string(),
            msg: Binary::new(raw.to_vec()),
            funds: vec![],
        })];

        assert_eq!(owner_from_cosmos_msgs(&msgs), Some("n1alice".to_string()));
    }
}
