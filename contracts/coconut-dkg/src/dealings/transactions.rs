// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::dealings::storage::{
    metadata_exists, must_read_metadata, store_metadata, StoredDealing,
};
use crate::epoch_state::storage::{CURRENT_EPOCH, INITIAL_REPLACEMENT_DATA};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::STATE;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, Storage};
use nym_coconut_dkg_common::dealing::{
    DealingChunkInfo, DealingMetadata, PartialContractDealing, MAX_DEALING_CHUNKS,
};
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, EpochState};

// make sure the epoch is in the dealing exchange and the message sender is a valid dealer for this epoch
fn ensure_permission(
    storage: &dyn Storage,
    sender: &Addr,
    resharing: bool,
) -> Result<(), ContractError> {
    check_epoch_state(storage, EpochState::DealingExchange { resharing })?;

    // ensure the sender is a dealer
    if dealers_storage::current_dealers()
        .may_load(storage, sender)?
        .is_none()
    {
        return Err(ContractError::NotADealer);
    }
    if resharing
        && !INITIAL_REPLACEMENT_DATA
            .load(storage)?
            .initial_dealers
            .contains(sender)
    {
        return Err(ContractError::NotAnInitialDealer);
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
    ensure_permission(deps.storage, &info.sender, resharing)?;

    let state = STATE.load(deps.storage)?;
    let epoch = CURRENT_EPOCH.load(deps.storage)?;

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
    resharing: bool,
) -> Result<Response, ContractError> {
    ensure_permission(deps.storage, &info.sender, resharing)?;

    let epoch = CURRENT_EPOCH.load(deps.storage)?;

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
    if submission_status.info.size != chunk.data.len() {
        return Err(ContractError::InconsistentChunkLength {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            dealing_index: chunk.dealing_index,
            chunk_index: chunk.chunk_index,
            metadata_length: submission_status.info.size,
            received: chunk.data.len(),
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

    Ok(Response::new())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::storage::CURRENT_EPOCH;
    use crate::epoch_state::transactions::{advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::fixtures::{
        dealer_details_fixture, dealing_metadata_fixture, partial_dealing_fixture,
    };
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{add_fixture_dealer, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::{
        ContractSafeBytes, InitialReplacementData, TimeConfiguration,
    };

    #[test]
    fn invalid_commit_dealing_chunk() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let owner = Addr::unchecked("owner1");
        let info = mock_info(owner.as_str(), &[]);
        let dealing = partial_dealing_fixture();

        let ret = try_commit_dealings_chunk(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            dealing.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::PublicKeySubmission { resharing: false }.to_string(),
                expected_state: EpochState::DealingExchange { resharing: false }.to_string()
            }
        );

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        add_fixture_dealer(deps.as_mut());
        advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let ret = try_commit_dealings_chunk(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            dealing.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(ret, ContractError::NotADealer);

        let dealer_details = DealerDetails {
            address: owner.clone(),
            bte_public_key_with_proof: String::new(),
            ed25519_identity: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        dealers_storage::current_dealers()
            .save(deps.as_mut().storage, &owner, &dealer_details)
            .unwrap();

        // assume we're in resharing mode
        CURRENT_EPOCH
            .update::<_, ContractError>(deps.as_mut().storage, |mut epoch| {
                epoch.state = EpochState::DealingExchange { resharing: true };
                Ok(epoch)
            })
            .unwrap();
        INITIAL_REPLACEMENT_DATA
            .save(
                deps.as_mut().storage,
                &InitialReplacementData {
                    initial_dealers: vec![],
                    initial_height: 1,
                },
            )
            .unwrap();
        let ret = try_commit_dealings_chunk(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            dealing.clone(),
            true,
        )
        .unwrap_err();
        assert_eq!(ret, ContractError::NotAnInitialDealer);

        INITIAL_REPLACEMENT_DATA
            .update::<_, ContractError>(deps.as_mut().storage, |mut data| {
                data.initial_dealers = vec![dealer_details_fixture(1).address];
                Ok(data)
            })
            .unwrap();

        // back to 'normal' mode
        CURRENT_EPOCH
            .update::<_, ContractError>(deps.as_mut().storage, |mut epoch| {
                epoch.state = EpochState::DealingExchange { resharing: false };
                Ok(epoch)
            })
            .unwrap();

        // TODO: test case: no metadata
        //
        //

        // add dealing metadata
        try_submit_dealings_metadata(
            deps.as_mut(),
            info.clone(),
            0,
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
            false,
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
        let ret = try_commit_dealings_chunk(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            dealing.clone(),
            false,
        );
        assert!(ret.is_ok());

        // duplicate dealing
        let ret = try_commit_dealings_chunk(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            dealing.clone(),
            false,
        )
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
        CURRENT_EPOCH
            .update::<_, ContractError>(deps.as_mut().storage, |mut epoch| {
                epoch.epoch_id += 1;
                Ok(epoch)
            })
            .unwrap();

        try_submit_dealings_metadata(
            deps.as_mut(),
            info.clone(),
            0,
            dealing_metadata_fixture(),
            false,
        )
        .unwrap();

        let ret = try_commit_dealings_chunk(deps.as_mut(), env, info, dealing.clone(), false);
        assert!(ret.is_ok());
    }
}
