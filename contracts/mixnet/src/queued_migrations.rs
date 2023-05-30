// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage as interval_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::DepsMut;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::PendingMixNodeChanges;
use mixnet_contract_common::PendingEpochEventKind;
use std::collections::BTreeMap;

pub fn insert_pending_pledge_changes(deps: DepsMut<'_>) -> Result<(), MixnetContractError> {
    let last_executed = interval_storage::LAST_PROCESSED_EPOCH_EVENT.load(deps.storage)?;
    let last_inserted = interval_storage::EPOCH_EVENT_ID_COUNTER.load(deps.storage)?;

    let mut new_pending = BTreeMap::new();

    for event_id in last_executed + 1..=last_inserted {
        let event = interval_storage::PENDING_EPOCH_EVENTS.load(deps.storage, event_id)?;
        match event.kind {
            PendingEpochEventKind::PledgeMore { mix_id, .. }
            | PendingEpochEventKind::DecreasePledge { mix_id, .. } => {
                if new_pending.insert(mix_id, event_id).is_some() {
                    return Err(MixnetContractError::FailedMigration { comment: format!("mixnode {mix_id} has more than a single pledge change pending for this epoch. Run this migration again after the epoch has finished.") });
                }
            }
            _ => (),
        }
    }

    for (mix_id, event_id) in new_pending {
        mixnodes_storage::PENDING_MIXNODE_CHANGES.save(
            deps.storage,
            mix_id,
            &PendingMixNodeChanges {
                pledge_change: Some(event_id),
            },
        )?;
    }

    Ok(())
}
