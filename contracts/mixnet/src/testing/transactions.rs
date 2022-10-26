// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::transactions::{
    perform_pending_epoch_actions, perform_pending_interval_actions,
};
use cosmwasm_std::{DepsMut, Env, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_pending_epoch_events_execution_event, new_pending_interval_events_execution_event,
    new_reconcile_pending_events,
};

pub fn try_resolve_all_pending_events(
    mut deps: DepsMut<'_>,
    env: Env,
    mut limit: Option<u32>,
) -> Result<Response, MixnetContractError> {
    let mut response = Response::new().add_event(new_reconcile_pending_events());

    // epoch events
    let (mut sub_response, executed) = perform_pending_epoch_actions(deps.branch(), &env, limit)?;
    response.messages.append(&mut sub_response.messages);
    response.attributes.append(&mut sub_response.attributes);
    response.events.append(&mut sub_response.events);
    response
        .events
        .push(new_pending_epoch_events_execution_event(executed));

    limit = limit.map(|l| l - executed);

    // interval events
    let (mut sub_response, executed) =
        perform_pending_interval_actions(deps.branch(), &env, limit)?;
    response.messages.append(&mut sub_response.messages);
    response.attributes.append(&mut sub_response.attributes);
    response.events.append(&mut sub_response.events);
    response
        .events
        .push(new_pending_interval_events_execution_event(executed));

    Ok(response)
}
