// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTRACT_STATE_KEY;
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{Addr, DepsMut, Order, Storage};
use cw_storage_plus::Item;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{ContractState, ContractStateParams, PendingEpochEventKind};
use serde::{Deserialize, Serialize};

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

pub(crate) fn explicit_contract_admin(deps: DepsMut) -> Result<(), MixnetContractError> {
    #[derive(Deserialize, Serialize)]
    pub struct OldContractState {
        pub owner: Addr,
        pub rewarding_validator_address: Addr,
        pub vesting_contract_address: Addr,
        pub rewarding_denom: String,
        pub params: ContractStateParams,
    }

    let old_state_storage = Item::<OldContractState>::new(CONTRACT_STATE_KEY);
    let old_state = old_state_storage.load(deps.storage)?;

    mixnet_params_storage::initialise_storage(
        deps,
        ContractState {
            rewarding_validator_address: old_state.rewarding_validator_address,
            vesting_contract_address: old_state.vesting_contract_address,
            rewarding_denom: old_state.rewarding_denom,
            params: old_state.params,
        },
        old_state.owner,
    )?;
    Ok(())
}
