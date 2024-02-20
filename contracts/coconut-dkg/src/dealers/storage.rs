// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::Dealer;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};
use nym_coconut_dkg_common::types::{DealerDetails, DealerRegistrationDetails, EpochId, NodeIndex};

pub(crate) const DEALER_INDICES_PAGE_MAX_LIMIT: u32 = 80;
pub(crate) const DEALER_INDICES_PAGE_DEFAULT_LIMIT: u32 = 40;

pub(crate) const DEALERS_PAGE_MAX_LIMIT: u32 = 25;
pub(crate) const DEALERS_PAGE_DEFAULT_LIMIT: u32 = 10;

pub(crate) const NODE_INDEX_COUNTER: Item<NodeIndex> = Item::new("node_index_counter");

pub(crate) const DEALERS_INDICES: Map<Dealer, NodeIndex> = Map::new("dealer_index");

pub(crate) const EPOCH_DEALERS_MAP: Map<(EpochId, Dealer), DealerRegistrationDetails> =
    Map::new("epoch_dealers");

/// Attempts to retrieve a pre-assign node index associated with given dealer.
/// If one doesn't exist, a new one is assigned.
pub(crate) fn get_or_assign_index(
    storage: &mut dyn Storage,
    dealer: Dealer,
) -> StdResult<NodeIndex> {
    if let Some(index) = DEALERS_INDICES.may_load(storage, dealer)? {
        return Ok(index);
    }
    let index = next_node_index(storage)?;
    DEALERS_INDICES.save(storage, dealer, &index)?;
    Ok(index)
}

pub(crate) fn save_dealer_details_if_not_a_dealer(
    storage: &mut dyn Storage,
    dealer: Dealer,
    epoch_id: EpochId,
    details: DealerRegistrationDetails,
) -> Result<(), ContractError> {
    if EPOCH_DEALERS_MAP.has(storage, (epoch_id, dealer)) {
        return Err(ContractError::AlreadyADealer);
    }
    EPOCH_DEALERS_MAP.save(storage, (epoch_id, dealer), &details)?;
    Ok(())
}

pub(crate) fn ensure_dealer(
    storage: &dyn Storage,
    dealer: Dealer,
    epoch_id: EpochId,
) -> Result<(), ContractError> {
    if !is_dealer(storage, dealer, epoch_id) {
        return Err(ContractError::NotADealer { epoch_id });
    }
    Ok(())
}

pub(crate) fn is_dealer(storage: &dyn Storage, dealer: Dealer, epoch_id: EpochId) -> bool {
    EPOCH_DEALERS_MAP.has(storage, (epoch_id, dealer))
}

// note: `epoch_id` is provided purely for the error message. it has nothing to do with storage retrieval
pub(crate) fn get_dealer_index(
    storage: &dyn Storage,
    dealer: Dealer,
    epoch_id: EpochId,
) -> Result<NodeIndex, ContractError> {
    DEALERS_INDICES
        .may_load(storage, dealer)?
        .ok_or(ContractError::NotADealer { epoch_id })
}

pub(crate) fn get_registration_details(
    storage: &dyn Storage,
    dealer: Dealer,
    epoch_id: EpochId,
) -> Result<DealerRegistrationDetails, ContractError> {
    EPOCH_DEALERS_MAP
        .may_load(storage, (epoch_id, dealer))?
        .ok_or(ContractError::NotADealer { epoch_id })
}

pub(crate) fn get_dealer_details(
    storage: &dyn Storage,
    dealer: Dealer,
    epoch_id: EpochId,
) -> Result<DealerDetails, ContractError> {
    let registration_details = get_registration_details(storage, dealer, epoch_id)?;
    let assigned_index = get_dealer_index(storage, dealer, epoch_id)?;
    Ok(DealerDetails {
        address: dealer.to_owned(),
        bte_public_key_with_proof: registration_details.bte_public_key_with_proof,
        ed25519_identity: registration_details.ed25519_identity,
        announce_address: registration_details.announce_address,
        assigned_index,
    })
}

pub(crate) fn next_node_index(store: &mut dyn Storage) -> StdResult<NodeIndex> {
    // make sure we don't start from 0, otherwise all the crypto breaks (kinda)
    let id: NodeIndex = NODE_INDEX_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NODE_INDEX_COUNTER.save(store, &id)?;
    Ok(id)
}
