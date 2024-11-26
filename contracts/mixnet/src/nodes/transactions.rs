// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::compat::helpers::{
    ensure_can_decrease_pledge, ensure_can_increase_pledge, ensure_can_modify_cost_params,
};
use crate::interval::storage as interval_storage;
use crate::interval::storage::push_new_interval_event;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::nodes::helpers::{must_get_node_bond_by_owner, save_new_nymnode};
use crate::nodes::signature_helpers::verify_bonding_signature;
use crate::nodes::storage;
use crate::nodes::storage::set_unbonding;
use crate::signing::storage as signing_storage;
use crate::support::helpers::{
    ensure_epoch_in_progress_state, ensure_no_existing_bond, ensure_operating_cost_within_range,
    ensure_profit_margin_within_range, validate_pledge,
};
use cosmwasm_std::{coin, Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_nym_node_bonding_event, new_pending_cost_params_update_event,
    new_pending_nym_node_unbonding_event, new_pending_pledge_decrease_event,
    new_pending_pledge_increase_event,
};
use mixnet_contract_common::nym_node::{NodeConfigUpdate, NymNode};
use mixnet_contract_common::{
    NodeCostParams, NymNodeBondingPayload, NymNodeDetails, PendingEpochEventKind,
    PendingIntervalEventKind,
};
use nym_contracts_common::signing::{MessageSignature, SigningPurpose};
use serde::Serialize;

pub fn try_add_nym_node(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    nym_node: NymNode,
    cost_params: NodeCostParams,
    owner_signature: MessageSignature,
) -> Result<Response, MixnetContractError> {
    // TODO: here be backwards compatibility checks for making sure there's no pre-existing mixnode/gateway

    add_nym_node_inner(
        deps,
        env,
        info,
        nym_node.clone(),
        cost_params.clone(),
        owner_signature,
        NymNodeBondingPayload::new(nym_node, cost_params),
    )
}

// allow bonding nym-node through mixnode/gateway entry points for backwards compatibility,
// and make sure to check correct signatures
pub(crate) fn add_nym_node_inner<T>(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    nym_node: NymNode,
    cost_params: NodeCostParams,
    owner_signature: MessageSignature,
    signed_message_payload: T,
) -> Result<Response, MixnetContractError>
where
    T: SigningPurpose + Serialize,
{
    // ensure the provided values for host and public key are not insane
    nym_node.ensure_host_in_range()?;
    nym_node.naive_ensure_valid_pubkey()?;

    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::minimum_node_pledge(deps.storage)?;
    let pledge = validate_pledge(info.funds, minimum_pledge)?;

    // ensure the profit margin is within the defined range
    ensure_profit_margin_within_range(deps.storage, cost_params.profit_margin_percent)?;

    // ensure the operating cost is within the defined range
    ensure_operating_cost_within_range(deps.storage, &cost_params.interval_operating_cost)?;

    // if the client has an active bonded [legacy] mixnode, [legacy] gateway or a nym-node, don't allow bonding
    // note that this has to be done explicitly as `UniqueIndex` constraint would not protect us
    // against attempting to use different node types (i.e. gateways and mixnodes)
    ensure_no_existing_bond(&info.sender, deps.storage)?;

    // there's no need to explicitly check whether there already exists nymnode with the same
    // identity as this is going to be done implicitly when attempting to save
    // the bond information due to `UniqueIndex` constraint defined on that field.

    // check if this sender actually owns the node by checking the signature
    verify_bonding_signature(
        deps.as_ref(),
        info.sender.clone(),
        &nym_node.identity_key,
        pledge.clone(),
        signed_message_payload,
        owner_signature,
    )?;

    // update the signing nonce associated with this sender so that the future signature would be made on the new value
    signing_storage::increment_signing_nonce(deps.storage, info.sender.clone())?;

    let node_identity = nym_node.identity_key.clone();
    let node_id = save_new_nymnode(
        deps.storage,
        env.block.height,
        nym_node,
        cost_params,
        info.sender.clone(),
        pledge.clone(),
    )?;

    Ok(Response::new().add_event(new_nym_node_bonding_event(
        &info.sender,
        &pledge,
        &node_identity,
        node_id,
    )))
}

pub(crate) fn try_remove_nym_node(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_node_bond_by_owner(deps.storage, &info.sender)?;
    let pending_changes =
        storage::PENDING_NYMNODE_CHANGES.load(deps.storage, existing_bond.node_id)?;

    // unbonding is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    // see if the proxy matches
    existing_bond.ensure_bonded()?;

    // if there are any pending requests to change the pledge, wait for them to resolve before allowing the unbonding
    pending_changes.ensure_no_pending_pledge_changes()?;

    // set `is_unbonding` field
    set_unbonding(deps.storage, &existing_bond)?;

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::UnbondNymNode {
        node_id: existing_bond.node_id,
    };
    interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;

    Ok(
        Response::new().add_event(new_pending_nym_node_unbonding_event(
            &existing_bond.owner,
            existing_bond.identity(),
            existing_bond.node_id,
        )),
    )
}

pub(crate) fn try_update_node_config(
    deps: DepsMut<'_>,
    info: MessageInfo,
    update: NodeConfigUpdate,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_node_bond_by_owner(deps.storage, &info.sender)?;
    existing_bond.ensure_bonded()?;

    let mut updated_bond = existing_bond.clone();

    if let Some(updated_host) = update.host {
        updated_bond.node.host = updated_host;
    }

    if let Some(updated_custom_http_port) = update.custom_http_port {
        updated_bond.node.custom_http_port = Some(updated_custom_http_port);
    }

    if update.restore_default_http_port {
        updated_bond.node.custom_http_port = None
    }

    storage::nym_nodes().replace(
        deps.storage,
        existing_bond.node_id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    Ok(Response::new())
}

pub(crate) fn try_increase_nym_node_pledge(
    deps: DepsMut<'_>,
    env: Env,
    increase: Vec<Coin>,
    node_details: NymNodeDetails,
) -> Result<Response, MixnetContractError> {
    let mut pending_changes = node_details.pending_changes;
    let node_id = node_details.node_id();

    ensure_can_increase_pledge(deps.storage, &node_details)?;

    let rewarding_denom = &node_details.original_pledge().denom;
    let pledge_increase = validate_pledge(increase, coin(1, rewarding_denom))?;

    let cosmos_event = new_pending_pledge_increase_event(node_id, &pledge_increase);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::NymNodePledgeMore {
        node_id,
        amount: pledge_increase,
    };
    let epoch_event_id = interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;
    pending_changes.pledge_change = Some(epoch_event_id);
    storage::PENDING_NYMNODE_CHANGES.save(deps.storage, node_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn try_decrease_nym_node_pledge(
    deps: DepsMut<'_>,
    env: Env,
    decrease_by: Coin,
    node_details: NymNodeDetails,
) -> Result<Response, MixnetContractError> {
    let mut pending_changes = node_details.pending_changes;
    let node_id = node_details.node_id();

    ensure_can_decrease_pledge(deps.storage, &node_details, &decrease_by)?;

    let cosmos_event = new_pending_pledge_decrease_event(node_id, &decrease_by);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::NymNodeDecreasePledge {
        node_id,
        decrease_by,
    };
    let epoch_event_id = interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;
    pending_changes.pledge_change = Some(epoch_event_id);
    storage::PENDING_NYMNODE_CHANGES.save(deps.storage, node_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn try_update_nym_node_cost_params(
    deps: DepsMut,
    env: Env,
    new_costs: NodeCostParams,
    node_details: NymNodeDetails,
) -> Result<Response, MixnetContractError> {
    let mut pending_changes = node_details.pending_changes;
    let node_id = node_details.node_id();

    ensure_can_modify_cost_params(deps.storage, &node_details)?;

    let cosmos_event = new_pending_cost_params_update_event(node_id, &new_costs);

    // push the interval event
    let interval_event = PendingIntervalEventKind::ChangeNymNodeCostParams { node_id, new_costs };
    let interval_event_id = push_new_interval_event(deps.storage, &env, interval_event)?;
    pending_changes.cost_params_change = Some(interval_event_id);
    storage::PENDING_NYMNODE_CHANGES.save(deps.storage, node_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}
