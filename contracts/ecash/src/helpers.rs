// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{BLACKLIST_PROPOSAL_REPLY_ID, REDEMPTION_PROPOSAL_REPLY_ID};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Reply, StdError, StdResult, SubMsg, SubMsgResult,
    WasmMsg,
};
use cw4::Cw4Contract;
use nym_contracts_common::events::try_find_attribute;
use nym_ecash_contract_common::events::{PROPOSAL_ID_ATTRIBUTE_NAME, WASM_EVENT_NAME};
use nym_ecash_contract_common::redeem_credential::BATCH_REDEMPTION_PROPOSAL_TITLE;
use nym_ecash_contract_common::{msg::ExecuteMsg, EcashContractError};
use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;
use serde::{Deserialize, Serialize};

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crate:nym-ecash";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runtime configuration persisted at the `"config"` storage key. Set on
/// instantiation; the only field mutable through an execute path is
/// `deposit_amount` (via `UpdateDefaultDepositValue`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// cw4 group contract referenced by the (stubbed) blacklist proposal flow.
    pub group_addr: Cw4Contract,

    /// Cosmos SDK address reserved for the future pool-contract transition.
    /// Currently never debited.
    pub holding_account: Addr,

    /// Default per-deposit price. Whitelisted senders may pay this *or* their
    /// per-account reduced amount; everyone else must pay this exact value.
    #[serde(alias = "default_deposit_amount")]
    pub deposit_amount: Coin,
}

/// Blacklist storage key - a bs58-encoded ed25519 public key.
pub(crate) type BlacklistKey = String;

/// Multisig-issued `proposal_id` returned via the reply pipeline.
pub(crate) type ProposalId = u64;

/// Hard upper bound on the `limit` accepted by paginated blacklist queries.
pub(crate) const BLACKLIST_PAGE_MAX_LIMIT: u32 = 75;
/// Default `limit` for paginated blacklist queries when the caller passes None.
pub(crate) const BLACKLIST_PAGE_DEFAULT_LIMIT: u32 = 50;

/// Hard upper bound on the `limit` accepted by paginated deposit queries.
pub(crate) const DEPOSITS_PAGE_MAX_LIMIT: u32 = 100;
/// Default `limit` for paginated deposit queries when the caller passes None.
pub(crate) const DEPOSITS_PAGE_DEFAULT_LIMIT: u32 = 50;

/// Build the cw3 `Propose` SubMsg dispatched by `RequestRedemption`. The
/// embedded message is a self-targeted `RedeemTickets` call that the multisig
/// will execute once the proposal passes.
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
        msg: to_json_binary(&release_funds_req)?,
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
        msg: to_json_binary(&req)?,
        funds: vec![],
    });

    let submsg = SubMsg::reply_always(msg, REDEMPTION_PROPOSAL_REPLY_ID);

    Ok(submsg)
}

/// Build the cw3 `Propose` SubMsg for the blacklist flow. **Dead path**: not
/// reachable from any public ExecuteMsg today; preserved for the redesign.
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
        msg: to_json_binary(&blacklist_req)?,
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
        msg: to_json_binary(&req)?,
        funds: vec![],
    });

    let submsg = SubMsg::reply_always(msg, BLACKLIST_PROPOSAL_REPLY_ID);

    Ok(submsg)
}

/// Extract the multisig-issued `proposal_id` from a cw3 `Propose` reply.
/// Surfaces `MissingProposalId` / `MalformedProposalId` for the typed-failure
/// cases the reply handler distinguishes.
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
