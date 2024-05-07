// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, Reply, StdError, StdResult, SubMsg, SubMsgResult, WasmMsg,
};
use cw4::Cw4Contract;
use nym_contracts_common::events::try_find_attribute;
use nym_ecash_contract_common::events::{
    PROPOSAL_ID_ATTRIBUTE_NAME, REDEMPTION_PROPOSAL_REPLY_ID, WASM_EVENT_NAME,
};
use nym_ecash_contract_common::redeem_credential::BATCH_REDEMPTION_PROPOSAL_TITLE;
use nym_ecash_contract_common::{
    events::BLACKLIST_PROPOSAL_REPLY_ID, msg::ExecuteMsg, EcashContractError,
};
use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;
use serde::{Deserialize, Serialize};

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crate:nym-ecash";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub group_addr: Cw4Contract,
    pub mix_denom: String,
    pub holding_account: Addr,

    pub redemption_gateway_share: Decimal,
}

//type aliases for easier reasoning
pub(crate) type BlacklistKey = String;
pub(crate) type ProposalId = u64;

// paged retrieval limits for all blacklist queries and transactions
pub(crate) const BLACKLIST_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const BLACKLIST_PAGE_DEFAULT_LIMIT: u32 = 50;

// paged retrieval limits for all deposit queries and transactions
pub(crate) const DEPOSITS_PAGE_MAX_LIMIT: u32 = 100;
pub(crate) const DEPOSITS_PAGE_DEFAULT_LIMIT: u32 = 50;

pub(crate) fn create_batch_redemption_proposal(
    tickets_digest: String,
    gw: String,
    n: u16,
    ecash_bandwidth_address: String,
    multisig_addr: String,
) -> StdResult<SubMsg> {
    let release_funds_req = ExecuteMsg::RedeemTickets { n, gw };
    let release_funds_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: ecash_bandwidth_address,
        msg: to_binary(&release_funds_req)?,
        funds: vec![],
    });
    let req = MultisigExecuteMsg::Propose {
        title: BATCH_REDEMPTION_PROPOSAL_TITLE.to_string(),
        description: tickets_digest,
        msgs: vec![release_funds_msg],
        latest: None,
    };
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: multisig_addr,
        msg: to_binary(&req)?,
        funds: vec![],
    });

    let submsg = SubMsg::reply_always(msg, REDEMPTION_PROPOSAL_REPLY_ID);

    Ok(submsg)
}

pub(crate) fn create_blacklist_proposal(
    public_key: String,
    ecash_bandwidth_address: String,
    multisig_addr: String,
) -> StdResult<SubMsg> {
    let blacklist_req = ExecuteMsg::AddToBlacklist {
        public_key: public_key.clone(),
    };
    let blacklist_req_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: ecash_bandwidth_address,
        msg: to_binary(&blacklist_req)?,
        funds: vec![],
    });
    let req = MultisigExecuteMsg::Propose {
        title: String::from("Add to blacklist, as ordered by Ecash Bandwidth Contract"),
        description: public_key,
        msgs: vec![blacklist_req_msg],
        latest: None,
    };
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: multisig_addr,
        msg: to_binary(&req)?,
        funds: vec![],
    });

    let submsg = SubMsg::reply_always(msg, BLACKLIST_PROPOSAL_REPLY_ID);

    Ok(submsg)
}

pub(crate) trait MultisigReply {
    fn multisig_proposal_id(&self) -> Result<u64, EcashContractError>;
}

impl MultisigReply for Reply {
    fn multisig_proposal_id(&self) -> Result<u64, EcashContractError> {
        match &self.result {
            SubMsgResult::Ok(res) => {
                let proposal_id: u64 =
                    try_find_attribute(&res.events, WASM_EVENT_NAME, PROPOSAL_ID_ATTRIBUTE_NAME)
                        .ok_or(EcashContractError::MissingProposalId)?
                        .map_err(|_| EcashContractError::MalformedProposalId)?;
                Ok(proposal_id)
            }
            SubMsgResult::Err(reply_err) => Err(StdError::generic_err(reply_err).into()),
        }
    }
}
