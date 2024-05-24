// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage as interval_storage;
use cosmwasm_std::{DepsMut, Order, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::PendingEpochEventKind;

fn ensure_no_pending_proxy_events(storage: &dyn Storage) -> Result<(), MixnetContractError> {
    let last_executed = interval_storage::LAST_PROCESSED_EPOCH_EVENT.load(storage)?;
    let last_inserted = interval_storage::EPOCH_EVENT_ID_COUNTER.load(storage)?;

    // no pending events
    if last_executed == last_inserted {
        return Ok(());
    }

    for maybe_event in
        interval_storage::PENDING_EPOCH_EVENTS.range(storage, None, None, Order::Ascending)
    {
        let (id, event_data) = maybe_event?;
        match event_data.kind {
            PendingEpochEventKind::Delegate { proxy, .. } => {
                if proxy.is_some() {
                    return Err(MixnetContractError::FailedMigration {
                        comment: format!(
                            "there is a pending vesting contract delegation with id {id}"
                        ),
                    });
                }
            }
            PendingEpochEventKind::Undelegate { proxy, .. } => {
                if proxy.is_some() {
                    return Err(MixnetContractError::FailedMigration {
                        comment: format!(
                            "there is a pending vesting contract undelegation with id {id}"
                        ),
                    });
                }
            }
            _ => continue,
        }
    }
    Ok(())
}

pub(crate) fn vesting_purge(deps: DepsMut) -> Result<(), MixnetContractError> {
    ensure_no_pending_proxy_events(deps.storage)?;

    Ok(())
}
