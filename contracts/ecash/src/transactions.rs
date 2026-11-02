// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! State-mutating execute and reply handlers.

use crate::helpers::{create_batch_redemption_proposal, MultisigReply, ProposalId};
use crate::storage::ECASH_CONTRACT_STORAGE;
use cosmwasm_std::{
    to_json_binary, Coin, Deps, DepsMut, Env, Event, MessageInfo, Reply, Response, StdResult,
};
use cw3::ProposalResponse;
use nym_ecash_contract_common::blacklist::Blacklisting;
use nym_ecash_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ID, PROPOSAL_ID_ATTRIBUTE_NAME,
};
use nym_ecash_contract_common::EcashContractError;
use nym_multisig_contract_common::msg::QueryMsg as MultisigQueryMsg;

fn query_multisig_proposal(
    deps: Deps,
    proposal_id: ProposalId,
) -> Result<ProposalResponse, EcashContractError> {
    let msg = MultisigQueryMsg::Proposal { proposal_id };
    let multisig_addr = ECASH_CONTRACT_STORAGE.must_get_multisig_addr(deps)?;

    let proposal_response: ProposalResponse = deps.querier.query(
        &cosmwasm_std::QueryRequest::Wasm(cosmwasm_std::WasmQuery::Smart {
            contract_addr: multisig_addr.to_string(),
            msg: to_json_binary(&msg)?,
        }),
    )?;
    Ok(proposal_response)
}

/// Submit a deposit. Classifies the sent amount (default → reduced →
/// `WrongAmount`), bumps the relevant counters, persists the deposit, and
/// emits a `deposited-funds` event with the assigned id.
pub(crate) fn try_deposit_ticket_book_funds(
    deps: DepsMut,
    info: MessageInfo,
    identity_key: String,
) -> Result<Response, EcashContractError> {
    let default_deposit = ECASH_CONTRACT_STORAGE
        .config
        .load(deps.storage)?
        .deposit_amount;
    let reduced_deposit = ECASH_CONTRACT_STORAGE
        .reduced_deposits
        .may_load(deps.storage, info.sender.clone())?;

    let submitted = cw_utils::must_pay(&info, &default_deposit.denom)?;
    let mut funds = info.funds;

    // Whitelisted accounts may deposit at either their reduced price or the
    // default price. If the default price is sent, the deposit is treated as
    // a regular (non-reduced) deposit for statistics purposes.
    if submitted == default_deposit.amount {
        ECASH_CONTRACT_STORAGE
            .deposit_stats
            .new_default_deposit(deps.storage, &default_deposit)?;
    } else if let Some(reduced_deposit) = reduced_deposit.as_ref() {
        // can't do if let chaining due to outdated rustc used for building contracts
        if reduced_deposit.amount == submitted {
            ECASH_CONTRACT_STORAGE.deposit_stats.new_reduced_deposit(
                deps.storage,
                &info.sender,
                reduced_deposit,
            )?;
        } else {
            // we are allowed to send reduced amounts, but we sent the wrong amount
            return Err(EcashContractError::WrongAmount {
                // SAFETY: the call to `must_pay` ensured a single coin has been sent
                #[allow(clippy::unwrap_used)]
                received: funds.pop().unwrap(),
                amount: reduced_deposit.clone(),
            });
        }
    } else {
        // we sent wrong amount of tokens
        return Err(EcashContractError::WrongAmount {
            // SAFETY: the call to `must_pay` ensured a single coin has been sent
            #[allow(clippy::unwrap_used)]
            received: funds.pop().unwrap(),
            amount: default_deposit,
        });
    };

    // global total needed when migrating to the nym pool contract
    ECASH_CONTRACT_STORAGE
        .pool_counters
        .update(deps.storage, |mut counters| -> StdResult<_> {
            counters.total_deposited.amount += submitted;
            Ok(counters)
        })?;

    let deposit_id = ECASH_CONTRACT_STORAGE
        .deposits
        .save_deposit(deps.storage, identity_key)?;

    Ok(Response::new()
        .add_event(
            Event::new(DEPOSITED_FUNDS_EVENT_TYPE)
                .add_attribute(DEPOSIT_ID, deposit_id.to_string()),
        )
        .set_data(deposit_id.to_be_bytes()))
}

/// Dispatch a multisig `Propose` SubMsg for batch ticket redemption.
/// Validates `commitment_bs58` decodes to a 32-byte sha256 digest; the
/// actual transfer happens (via the embedded `RedeemTickets`) only after
/// the multisig approves.
pub(crate) fn try_request_redemption(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    commitment_bs58: String,
    number_of_tickets: u16,
) -> Result<Response, EcashContractError> {
    // basic validation of commitment to make sure it's a valid sha256 digest
    let Ok(digest) = bs58::decode(&commitment_bs58).into_vec() else {
        return Err(EcashContractError::MalformedRedemptionCommitment);
    };
    if digest.len() != 32 {
        return Err(EcashContractError::MalformedRedemptionCommitment);
    }

    let multisig_addr = ECASH_CONTRACT_STORAGE.must_get_multisig_addr(deps.as_ref())?;
    let msg = create_batch_redemption_proposal(
        commitment_bs58,
        info.sender.into_string(),
        number_of_tickets,
        env.contract.address.into_string(),
        multisig_addr.into_string(),
    )?;
    Ok(Response::new().add_submessage(msg))
}

/// **Dead code.** Legacy multisig-gated redemption that bumps a counter
/// and emits a `ticket_redemption` event with `moved_to_holding_account =
/// "false"`. No known consumer depends on the side effects; candidate for
/// removal in a follow-on breaking-schema change.
pub(crate) fn try_redeem_tickets(
    deps: DepsMut,
    info: MessageInfo,
    n: u16,
    gw: String,
) -> Result<Response, EcashContractError> {
    // preserve the gateway argument so that upon scraping the chain and going through transactions,
    // one could see which gateway attempted to redeem it.
    // in the long run it will be needed to determine work factor.
    let _ = gw;

    // only a mutlisig proposal can do that
    ECASH_CONTRACT_STORAGE
        .multisig
        .assert_admin(deps.as_ref(), &info.sender)?;

    ECASH_CONTRACT_STORAGE
        .pool_counters
        .update(deps.storage, |mut counters| -> StdResult<_> {
            counters.tickets_requested_and_not_redeemed += n as u64;
            Ok(counters)
        })?;

    Ok(Response::new().add_event(
        Event::new("ticket_redemption").add_attribute("moved_to_holding_account", "false"),
    ))
}

/// Transfer the contract admin role. Dispatches via the cw_controllers
/// `execute_update_admin` handshake; the sender-equality check happens
/// inside that call. The handler always forwards `Some(new_admin)` -
/// admin renunciation is not exposed.
pub(crate) fn try_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: String,
) -> Result<Response, EcashContractError> {
    let new_admin = deps.api.addr_validate(&admin)?;

    // note: the below performs validation to ensure the sender IS the current admin
    Ok(ECASH_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?)
}

/// Overwrite `Config::deposit_amount`. Admin-gated; trips
/// `TicketBookSizeChanged` if the snapshotted invariant diverged from the
/// crate constant, and `DepositBelowTicketBookSize` if the new amount is
/// below `TICKETBOOK_SIZE`.
pub(crate) fn try_update_default_deposit_value(
    deps: DepsMut,
    info: MessageInfo,
    new_deposit: Coin,
) -> Result<Response, EcashContractError> {
    // only current admin can do that
    ECASH_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    let ticket_book_size = ECASH_CONTRACT_STORAGE.get_ticketbook_size(deps.storage)?;
    if new_deposit.amount < cosmwasm_std::Uint128::from(ticket_book_size) {
        return Err(EcashContractError::DepositBelowTicketBookSize {
            amount: new_deposit.amount,
            ticket_book_size,
        });
    }

    let deposit_str = new_deposit.to_string();
    ECASH_CONTRACT_STORAGE
        .config
        .update(deps.storage, |mut cfg| -> StdResult<_> {
            cfg.deposit_amount = new_deposit;
            Ok(cfg)
        })?;
    Ok(Response::new().add_attribute("updated_deposit", deposit_str))
}

/// Set or overwrite the reduced-deposit price for a single address.
/// Admin-gated; enforces matching denom, strictly-less-than-default amount,
/// and amount at least the snapshotted ticketbook size.
pub(crate) fn try_set_reduced_deposit_price(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    deposit: Coin,
) -> Result<Response, EcashContractError> {
    ECASH_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    let addr = deps.api.addr_validate(&address)?;

    // the reduced price must be strictly less than the default to avoid
    // accidentally misconfiguring an address to pay more than everyone else
    let default = ECASH_CONTRACT_STORAGE
        .config
        .load(deps.storage)?
        .deposit_amount;
    if deposit.denom != default.denom {
        return Err(EcashContractError::InvalidReducedDepositDenom {
            expected: default.denom,
            got: deposit.denom.clone(),
        });
    }
    if deposit.amount >= default.amount {
        return Err(EcashContractError::ReducedDepositNotReduced {
            reduced: deposit.amount,
            default: default.amount,
        });
    }

    let ticket_book_size = ECASH_CONTRACT_STORAGE.get_ticketbook_size(deps.storage)?;
    if deposit.amount < cosmwasm_std::Uint128::from(ticket_book_size) {
        return Err(EcashContractError::DepositBelowTicketBookSize {
            amount: deposit.amount,
            ticket_book_size,
        });
    }

    ECASH_CONTRACT_STORAGE
        .reduced_deposits
        .save(deps.storage, addr, &deposit)?;

    Ok(Response::new()
        .add_attribute("action", "set_reduced_deposit_price")
        .add_attribute("address", address)
        .add_attribute("deposit", deposit.to_string()))
}

/// Removes the reduced deposit price for a given address, reverting them to
/// the default deposit amount. This is safe to call even if the address has
/// already deposited at the reduced price - their next deposit will simply
/// use the default price. Historical deposit statistics are not affected.
pub(crate) fn try_remove_reduced_deposit_price(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, EcashContractError> {
    ECASH_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    let addr = deps.api.addr_validate(&address)?;

    if !ECASH_CONTRACT_STORAGE
        .reduced_deposits
        .has(deps.storage, addr.clone())
    {
        return Err(EcashContractError::NoReducedDepositPrice { address });
    }

    ECASH_CONTRACT_STORAGE
        .reduced_deposits
        .remove(deps.storage, addr);

    Ok(Response::new()
        .add_attribute("action", "remove_reduced_deposit_price")
        .add_attribute("address", address))
}

/// **Stubbed.** Always returns `UnimplementedBlacklisting`. The
/// commented-out body shows the intended group-gated propose flow for the
/// blacklist redesign.
pub(crate) fn try_propose_to_blacklist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    public_key: String,
) -> Result<Response, EcashContractError> {
    let _ = (deps, env, info);
    let _ = public_key;
    Err(EcashContractError::UnimplementedBlacklisting)
    // let cfg = ECASH_CONTRACT_STORAGE.config.load(deps.storage)?;
    // cfg.group_addr
    //     .is_voting_member(&deps.querier, &info.sender, env.block.height)?
    //     .ok_or(EcashContractError::Unauthorized)?;
    //
    // if let Some(blacklisted) = ECASH_CONTRACT_STORAGE
    //     .blacklist
    //     .may_load(deps.storage, public_key.clone())?
    // {
    //     // return existing proposal id
    //     Ok(Response::new().add_attribute(
    //         PROPOSAL_ID_ATTRIBUTE_NAME,
    //         blacklisted.proposal_id.to_string(),
    //     ))
    // } else {
    //     let multisig_addr = ECASH_CONTRACT_STORAGE.must_get_multisig_addr(deps.as_ref())?;
    //     let msg = create_blacklist_proposal(
    //         public_key,
    //         env.contract.address.into_string(),
    //         multisig_addr.into_string(),
    //     )?;
    //     Ok(Response::new().add_submessage(msg))
    // }
}

/// **Stubbed.** Always returns `UnimplementedBlacklisting`. The
/// commented-out body shows the intended multisig-gated finalisation for
/// the blacklist redesign.
pub(crate) fn try_add_to_blacklist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    public_key: String,
) -> Result<Response, EcashContractError> {
    let _ = (deps, env, info);
    let _ = public_key;
    Err(EcashContractError::UnimplementedBlacklisting)
    // //Only by multisig contract, actually add public key to blacklist
    // ECASH_CONTRACT_STORAGE
    //     .multisig
    //     .assert_admin(deps.as_ref(), &info.sender)?;
    //
    // let mut blacklisting = ECASH_CONTRACT_STORAGE
    //     .blacklist
    //     .load(deps.storage, public_key.clone())?;
    // blacklisting.finalized_at_height = Some(env.block.height);
    // ECASH_CONTRACT_STORAGE
    //     .blacklist
    //     .save(deps.storage, public_key.clone(), &blacklisting)?;
    //
    // Ok(Response::new())
}

/// Reply handler for the (currently dead) blacklist propose flow.
/// Reachable from the dispatcher in theory but no public ExecuteMsg path
/// dispatches a SubMsg with `BLACKLIST_PROPOSAL_REPLY_ID` today.
pub(crate) fn try_handle_blacklist_proposal_reply(
    deps: DepsMut,
    msg: Reply,
) -> Result<Response, EcashContractError> {
    let proposal_id = msg.multisig_proposal_id()?;

    let proposal = query_multisig_proposal(deps.as_ref(), proposal_id)?;
    let public_key = proposal.description;
    ECASH_CONTRACT_STORAGE.blacklist.save(
        deps.storage,
        public_key,
        &Blacklisting::new(proposal_id),
    )?;

    // TODO: that `BLACKLIST_PROPOSAL_ID` might be redundant since it should be available from cw3 event
    Ok(Response::new().add_attribute(PROPOSAL_ID_ATTRIBUTE_NAME, proposal_id.to_string()))
}

/// Reply handler for the redemption propose flow. Captures the multisig
/// `proposal_id` from the SubMsg result and re-exposes it as the response
/// `set_data` payload (big-endian `u64`).
pub(crate) fn try_handle_redemption_proposal_reply(
    msg: Reply,
) -> Result<Response, EcashContractError> {
    let proposal_id = msg.multisig_proposal_id()?;

    // emit the proposal_id in the response data for easy client access and to make sure it can't be tampered with
    // (since it's included as part of block hash)

    Ok(Response::new().set_data(proposal_id.to_be_bytes()))
}
