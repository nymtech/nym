// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{to_binary, BankMsg, Coin, Event, Order, Reply, Response, StdError, StdResult};
use cw3::ProposalResponse;
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::{Bound, Item, Map};
use nym_ecash_contract_common::blacklist::{
    BlacklistedAccount, BlacklistedAccountResponse, PagedBlacklistedAccountResponse,
};
use nym_ecash_contract_common::events::{
    BLACKLIST_PROPOSAL_ID, BLACKLIST_PROPOSAL_REPLY_ID, DEPOSITED_FUNDS_EVENT_TYPE,
    DEPOSIT_IDENTITY_KEY, DEPOSIT_INFO, DEPOSIT_VALUE,
};

use nym_multisig_contract_common::msg::QueryMsg as MultisigQueryMsg;
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx, ReplyCtx};
use sylvia::{contract, entry_points};

use crate::errors::ContractError;
use crate::helpers::{self, BlacklistKey, Config, ProposalId, SerialNumber};
use crate::helpers::{
    BLACKLIST_PAGE_DEFAULT_LIMIT, BLACKLIST_PAGE_MAX_LIMIT, SPEND_CREDENTIAL_PAGE_DEFAULT_LIMIT,
    SPEND_CREDENTIAL_PAGE_MAX_LIMIT,
};
use nym_ecash_contract_common::events::{TICKET_BOOK_VALUE, TICKET_VALUE};
use nym_ecash_contract_common::spend_credential::{
    EcashSpentCredential, EcashSpentCredentialResponse, PagedEcashSpentCredentialResponse,
};

pub struct NymEcashContract<'a> {
    pub(crate) multisig: Admin<'a>,
    pub(crate) config: Item<'a, Config>,
    pub(crate) spent_credentials: Map<'a, SerialNumber, EcashSpentCredential>,
    pub(crate) blacklist: Map<'a, BlacklistKey, BlacklistedAccount>,
    pub(crate) blacklist_proposals: Map<'a, BlacklistKey, ProposalId>,
}

#[entry_points]
#[contract]
#[error(ContractError)]
impl NymEcashContract<'_> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            multisig: Admin::new("multisig"),
            config: Item::new("config"),
            spent_credentials: Map::new("spent_credentials"),
            blacklist: Map::new("blacklist"),
            blacklist_proposals: Map::new("blacklist_proposal"),
        }
    }

    #[msg(instantiate)]
    pub fn instantiate(
        &self,
        mut ctx: InstantiateCtx,
        multisig_addr: String,
        group_addr: String,
        mix_denom: String,
    ) -> Result<Response, ContractError> {
        let multisig_addr = ctx.deps.api.addr_validate(&multisig_addr)?;
        let group_addr = Cw4Contract(ctx.deps.api.addr_validate(&group_addr).map_err(|_| {
            ContractError::InvalidGroup {
                addr: group_addr.clone(),
            }
        })?);

        self.multisig
            .set(ctx.deps.branch(), Some(multisig_addr.clone()))?;
        let cfg = Config {
            multisig_addr,
            group_addr,
            mix_denom,
        };

        self.config.save(ctx.deps.storage, &cfg)?;

        Ok(Response::default())
    }

    /*==================
    ======QUERIES=======
    ==================*/
    #[msg(query)]
    pub fn get_all_spent_credentials(
        &self,
        ctx: QueryCtx,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> StdResult<PagedEcashSpentCredentialResponse> {
        let limit = limit
            .unwrap_or(SPEND_CREDENTIAL_PAGE_DEFAULT_LIMIT)
            .min(SPEND_CREDENTIAL_PAGE_MAX_LIMIT) as usize;

        let start = start_after.as_deref().map(Bound::exclusive);

        let nodes = self
            .spent_credentials
            .range(ctx.deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|res| res.map(|item| item.1))
            .collect::<StdResult<Vec<EcashSpentCredential>>>()?;

        let start_next_after = nodes
            .last()
            .map(|spend_credential| spend_credential.serial_number().to_string());

        Ok(PagedEcashSpentCredentialResponse::new(
            nodes,
            limit,
            start_next_after,
        ))
    }

    #[msg(query)]
    pub fn get_spent_credential(
        &self,
        ctx: QueryCtx,
        serial_number: String,
    ) -> StdResult<EcashSpentCredentialResponse> {
        let spend_credential = self
            .spent_credentials
            .may_load(ctx.deps.storage, serial_number)?;
        Ok(EcashSpentCredentialResponse::new(spend_credential))
    }

    #[msg(query)]
    pub fn get_blacklist(
        &self,
        ctx: QueryCtx,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> StdResult<PagedBlacklistedAccountResponse> {
        let limit = limit
            .unwrap_or(BLACKLIST_PAGE_DEFAULT_LIMIT)
            .min(BLACKLIST_PAGE_MAX_LIMIT) as usize;

        let start = start_after.as_deref().map(Bound::exclusive);

        let nodes = self
            .blacklist
            .range(ctx.deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|res| res.map(|item| item.1))
            .collect::<StdResult<Vec<BlacklistedAccount>>>()?;

        let start_next_after = nodes
            .last()
            .map(|account: &BlacklistedAccount| account.public_key().to_string());

        Ok(PagedBlacklistedAccountResponse::new(
            nodes,
            limit,
            start_next_after,
        ))
    }

    #[msg(query)]
    pub fn get_blacklisted_account(
        &self,
        ctx: QueryCtx,
        public_key: String,
    ) -> StdResult<BlacklistedAccountResponse> {
        let account = self.blacklist.may_load(ctx.deps.storage, public_key)?;
        Ok(BlacklistedAccountResponse::new(account))
    }

    /*=====================
    ======EXECUTIONS=======
    =====================*/

    #[msg(exec)]
    pub fn deposit_funds(
        &self,
        ctx: ExecCtx,
        deposit_info: String,
        identity_key: String,
        encryption_key: String,
    ) -> Result<Response, ContractError> {
        let mix_denom = self.config.load(ctx.deps.storage)?.mix_denom;
        let voucher_value = cw_utils::must_pay(&ctx.info, &mix_denom)?;

        if u128::from(voucher_value) != TICKET_BOOK_VALUE {
            return Err(ContractError::WrongAmount {
                amount: TICKET_BOOK_VALUE,
            });
        }

        let event = Event::new(DEPOSITED_FUNDS_EVENT_TYPE)
            .add_attribute(DEPOSIT_VALUE, voucher_value)
            .add_attribute(DEPOSIT_INFO, deposit_info)
            .add_attribute(DEPOSIT_IDENTITY_KEY, identity_key);
        Ok(Response::new().add_event(event))
    }

    #[msg(exec)]
    pub fn prepare_credential(
        &self,
        ctx: ExecCtx,
        serial_number: String,
        gateway_cosmos_address: String,
    ) -> StdResult<Response> {
        let cfg = self.config.load(ctx.deps.storage)?;

        let gateway_cosmos_address = ctx.deps.api.addr_validate(&gateway_cosmos_address)?;

        let msg = helpers::create_spend_proposal(
            serial_number.to_string(),
            gateway_cosmos_address.to_string(),
            ctx.env.contract.address.into_string(),
            cfg.multisig_addr.into_string(),
        )?;

        Ok(Response::new().add_message(msg))
    }

    #[msg(exec)]
    pub fn spend_credential(
        &self,
        ctx: ExecCtx,
        serial_number: String,
        gateway_cosmos_address: String,
    ) -> Result<Response, ContractError> {
        //only a mutlisig proposal can do that
        self.multisig
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let mix_denom = self.config.load(ctx.deps.storage)?.mix_denom;
        let ticket_fund = Coin::new(TICKET_VALUE, mix_denom.clone());

        let return_tokens = BankMsg::Send {
            to_address: gateway_cosmos_address.clone(),
            amount: vec![ticket_fund],
        };

        self.spent_credentials.save(
            ctx.deps.storage,
            serial_number.clone(),
            &EcashSpentCredential::new(serial_number, gateway_cosmos_address),
        )?;

        let response = Response::new().add_message(return_tokens);

        Ok(response)
    }

    #[msg(exec)]
    pub fn propose_to_blacklist(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<Response, ContractError> {
        let cfg = self.config.load(ctx.deps.storage)?;
        cfg.group_addr
            .is_voting_member(&ctx.deps.querier, &ctx.info.sender, ctx.env.block.height)?
            .ok_or(ContractError::Unauthorized)?;

        if let Some(blacklist_proposal_id) = self
            .blacklist_proposals
            .may_load(ctx.deps.storage, public_key.clone())?
        {
            Ok(Response::new()
                .add_attribute(BLACKLIST_PROPOSAL_ID, blacklist_proposal_id.to_string()))
        } else {
            let msg = helpers::create_blacklist_proposal(
                public_key,
                ctx.env.contract.address.into_string(),
                cfg.multisig_addr.into_string(),
            )?;
            Ok(Response::new().add_submessage(msg))
        }
    }

    #[msg(exec)]
    pub fn add_to_blacklist(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<Response, ContractError> {
        //Only by multisig contract, actually add public key to blacklist
        self.multisig
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        self.blacklist.save(
            ctx.deps.storage,
            public_key.clone(),
            &BlacklistedAccount::new(public_key, ctx.env.block),
        )?;
        Ok(Response::new())
    }

    /*=====================
    =========REPLY=========
    =====================*/
    #[msg(reply)]
    pub fn reply(&self, ctx: ReplyCtx, msg: Reply) -> Result<Response, ContractError> {
        match msg.id {
            BLACKLIST_PROPOSAL_REPLY_ID => self.handle_blacklist_proposal_reply(ctx, msg),
            id => Err(ContractError::Std(cosmwasm_std::StdError::GenericErr {
                msg: format!("Unknown reply Id {}", id),
            })),
        }
    }

    fn handle_blacklist_proposal_reply(
        &self,
        ctx: ReplyCtx,
        msg: Reply,
    ) -> Result<Response, ContractError> {
        let reply = msg.result.into_result().map_err(StdError::generic_err)?;
        let proposal_attribute = reply
            .events
            .iter()
            .find(|event| event.ty == "wasm")
            .ok_or(ContractError::ProposalError(
                "Wasm event not found".to_string(),
            ))?
            .attributes
            .iter()
            .find(|attr| attr.key == BLACKLIST_PROPOSAL_ID)
            .ok_or(ContractError::ProposalError(
                "Proposal id not found".to_string(),
            ))?;

        let proposal_id = proposal_attribute.value.parse::<u64>().map_err(|_| {
            ContractError::ProposalError(String::from("proposal id could not be parsed to u64"))
        })?;

        let cfg = self.config.load(ctx.deps.storage)?;
        let msg = MultisigQueryMsg::Proposal { proposal_id };
        let proposal_response: ProposalResponse = ctx.deps.querier.query(
            &cosmwasm_std::QueryRequest::Wasm(cosmwasm_std::WasmQuery::Smart {
                contract_addr: cfg.multisig_addr.to_string(),
                msg: to_binary(&msg)?,
            }),
        )?;
        let public_key = proposal_response.description;
        self.blacklist_proposals
            .save(ctx.deps.storage, public_key, &proposal_id)?;

        Ok(Response::new().add_attribute(BLACKLIST_PROPOSAL_ID, proposal_id.to_string()))
    }
}
