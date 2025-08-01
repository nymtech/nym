// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::ensure_dealer;
use crate::dealings::storage::{
    metadata_exists, must_read_metadata, store_metadata, StoredDealing,
};
use crate::epoch_state::storage::{load_current_epoch, save_epoch};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::STATE;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, Storage};
use nym_coconut_dkg_common::dealing::{
    DealingChunkInfo, DealingMetadata, PartialContractDealing, MAX_DEALING_CHUNKS,
};
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, EpochId, EpochState};

// make sure the epoch is in the dealing exchange and the message sender is a valid dealer for this epoch
fn ensure_permission(
    storage: &dyn Storage,
    sender: &Addr,
    current_epoch_id: EpochId,
    resharing: bool,
) -> Result<(), ContractError> {
    check_epoch_state(storage, EpochState::DealingExchange { resharing })?;

    // ensure the sender is a dealer for this epoch
    ensure_dealer(storage, sender, current_epoch_id)?;

    // if we're in resharing, make sure this sender has also been a dealer in the previous epoch
    if resharing {
        ensure_dealer(storage, sender, current_epoch_id.saturating_sub(1))?;
    }

    Ok(())
}

pub fn try_submit_dealings_metadata(
    deps: DepsMut,
    info: MessageInfo,
    dealing_index: DealingIndex,
    chunks: Vec<DealingChunkInfo>,
    resharing: bool,
) -> Result<Response, ContractError> {
    let epoch = load_current_epoch(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    ensure_permission(deps.storage, &info.sender, epoch.epoch_id, resharing)?;

    // don't allow overwriting existing metadata
    if metadata_exists(deps.storage, epoch.epoch_id, &info.sender, dealing_index) {
        return Err(ContractError::MetadataAlreadyExists {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index,
        });
    }

    // make sure the dealing index is in the allowed range
    // note: dealing indexing starts from 0
    if dealing_index >= state.key_size {
        return Err(ContractError::DealingOutOfRange {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            index: dealing_index,
            key_size: state.key_size,
        });
    }

    // make sure the metadata is not empty
    if chunks.is_empty() {
        return Err(ContractError::EmptyMetadata {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index,
        });
    }

    // make sure the chunks are non empty
    if chunks.iter().any(|c| c.size == 0) {
        return Err(ContractError::EmptyMetadata {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index,
        });
    }

    // make sure the number of dealing chunks is in the allowed range
    // to prevent somebody splitting their dealings into 10B chunks
    if chunks.len() > MAX_DEALING_CHUNKS {
        return Err(ContractError::TooFragmentedMetadata {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index,
            chunks: chunks.len(),
        });
    }

    // make sure all chunks, but the last one, have the same size
    // SAFETY: we checked for whether `chunks` is empty and returned an error in that case
    #[allow(clippy::unwrap_used)]
    let first_chunk_size = chunks.first().unwrap().size;

    for (chunk_index, chunk_info) in chunks.iter().enumerate().take(chunks.len() - 1) {
        if chunk_info.size != first_chunk_size {
            return Err(ContractError::UnevenChunkSplit {
                epoch_id: epoch.epoch_id,
                dealer: info.sender,
                dealing_index,
                chunk_index: chunk_index as ChunkIndex,
                first_chunk_size,
                size: chunk_info.size,
            });
        }
    }

    // finally, construct and store the metadata
    let metadata = DealingMetadata::new(dealing_index, chunks);

    store_metadata(
        deps.storage,
        epoch.epoch_id,
        &info.sender,
        dealing_index,
        &metadata,
    )?;

    Ok(Response::new())
}

pub fn try_commit_dealings_chunk(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    chunk: PartialContractDealing,
) -> Result<Response, ContractError> {
    // note: checking permissions is implicit as if the metadata exists,
    // the sender must have been allowed to submit it

    let mut epoch = load_current_epoch(deps.storage)?;

    // read meta
    let mut metadata = must_read_metadata(
        deps.storage,
        epoch.epoch_id,
        &info.sender,
        chunk.dealing_index,
    )?;

    // check if the received chunk is within the declared range
    let Some(submission_status) = metadata.submitted_chunks.get_mut(&chunk.chunk_index) else {
        return Err(ContractError::DealingChunkNotInMetadata {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index: chunk.dealing_index,
            chunk_index: chunk.chunk_index,
        });
    };

    // check if this dealer has already committed this particular dealing chunk
    if let Some(submission_height) = submission_status.status.submission_height {
        return Err(ContractError::DealingChunkAlreadyCommitted {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index: chunk.dealing_index,
            chunk_index: chunk.chunk_index,
            block_height: submission_height,
        });
    }

    // check if the received chunk has the specified size
    if submission_status.info.size != chunk.data.len() as u64 {
        return Err(ContractError::InconsistentChunkLength {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index: chunk.dealing_index,
            chunk_index: chunk.chunk_index,
            metadata_length: submission_status.info.size,
            received: chunk.data.len() as u64,
        });
    }

    // update the metadata
    submission_status.status.submission_height = Some(env.block.height);
    store_metadata(
        deps.storage,
        epoch.epoch_id,
        &info.sender,
        chunk.dealing_index,
        &metadata,
    )?;

    // store the dealing
    StoredDealing::save(deps.storage, epoch.epoch_id, &info.sender, chunk);

    // this is less than ideal since we have to iterate through all the chunks, but realistically,
    // there won't be a lot of them
    if metadata.is_complete() {
        epoch.state_progress.submitted_dealings += 1;
        save_epoch(deps.storage, env.block.height, &epoch)?;
    }

    Ok(Response::new())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::transactions::{try_advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::fixtures::{dealing_metadata_fixture, partial_dealing_fixture};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{add_current_dealer, re_register_dealer, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_env};
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::{ContractSafeBytes, TimeConfiguration};

    #[test]
    fn invalid_commit_dealing_chunk() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        let owner = deps.api.addr_make("owner1");
        let info = message_info(&owner, &[]);
        let chunk = partial_dealing_fixture();

        // no dealing metadata
        let ret =
            try_commit_dealings_chunk(deps.as_mut(), env.clone(), info.clone(), chunk.clone())
                .unwrap_err();
        assert_eq!(
            ret,
            ContractError::UnavailableDealingMetadata {
                epoch_id: 0,
                dealer: info.sender.clone(),
                dealing_index: chunk.dealing_index,
            }
        );

        // add dealing metadata
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let dealer_details = DealerDetails {
            address: owner.clone(),
            bte_public_key_with_proof: String::new(),
            ed25519_identity: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        add_current_dealer(deps.as_mut(), &dealer_details);

        try_submit_dealings_metadata(
            deps.as_mut(),
            info.clone(),
            chunk.dealing_index,
            dealing_metadata_fixture(),
            false,
        )
        .unwrap();

        // dealing chunk out of range
        let ret = try_commit_dealings_chunk(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            PartialContractDealing {
                dealing_index: 0,
                chunk_index: 42,
                data: ContractSafeBytes(vec![1, 2, 3]),
            },
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::DealingChunkNotInMetadata {
                epoch_id: 0,
                dealer: info.sender.clone(),
                dealing_index: 0,
                chunk_index: 42,
            }
        );

        // 'good' dealing
        let ret =
            try_commit_dealings_chunk(deps.as_mut(), env.clone(), info.clone(), chunk.clone());
        assert!(ret.is_ok());

        // duplicate dealing
        let ret =
            try_commit_dealings_chunk(deps.as_mut(), env.clone(), info.clone(), chunk.clone())
                .unwrap_err();
        assert_eq!(
            ret,
            ContractError::DealingChunkAlreadyCommitted {
                epoch_id: 0,
                dealer: info.sender.clone(),
                dealing_index: 0,
                chunk_index: 0,
                block_height: env.block.height,
            }
        );

        // same index, but next epoch
        let mut epoch = load_current_epoch(&deps.storage).unwrap();
        epoch.epoch_id += 1;
        save_epoch(deps.as_mut().storage, epoch.epoch_id, &epoch).unwrap();

        re_register_dealer(deps.as_mut(), &info.sender);

        try_submit_dealings_metadata(
            deps.as_mut(),
            info.clone(),
            0,
            dealing_metadata_fixture(),
            false,
        )
        .unwrap();

        let ret = try_commit_dealings_chunk(deps.as_mut(), env, info, chunk.clone());
        assert!(ret.is_ok());
    }
}
