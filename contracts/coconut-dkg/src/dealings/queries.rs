// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealings::storage::{StoredDealing, DEALINGS_METADATA};
use crate::state::storage::STATE;
use cosmwasm_std::{Deps, StdResult};
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkResponse, DealingChunkStatusResponse,
    DealingMetadataResponse, DealingStatus, DealingStatusResponse,
};
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, EpochId};
use std::collections::BTreeMap;

/// Get the metadata associated with the particular dealing
pub fn query_dealing_metadata(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
) -> StdResult<DealingMetadataResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let metadata = DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;

    Ok(DealingMetadataResponse {
        epoch_id,
        dealer,
        dealing_index,
        metadata,
    })
}

/// Get the status of all dealings of particular dealer for given epoch.
pub fn query_dealer_dealings_status(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
) -> StdResult<DealerDealingsStatusResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let state = STATE.load(deps.storage)?;

    let mut dealing_submission_status: BTreeMap<DealingIndex, DealingStatus> = BTreeMap::new();

    // Since our key size is in single digit range, querying all of this at once on chain is fine
    for dealing_index in 0..state.key_size {
        let metadata =
            DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;
        dealing_submission_status.insert(dealing_index, metadata.into());
    }

    Ok(DealerDealingsStatusResponse {
        epoch_id,
        dealer,
        all_dealings_fully_submitted: dealing_submission_status
            .values()
            .all(|d| d.fully_submitted),
        dealing_submission_status,
    })
}

/// Get the status of particular dealing, i.e. whether it has been fully submitted.
pub fn query_dealing_status(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
) -> StdResult<DealingStatusResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let metadata = DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;

    Ok(DealingStatusResponse {
        epoch_id,
        dealer,
        dealing_index,
        status: metadata.into(),
    })
}

/// Get the status of particular chunk, i.e. whether (and when) it has been fully submitted.
pub fn query_dealing_chunk_status(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
    chunk_index: ChunkIndex,
) -> StdResult<DealingChunkStatusResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let metadata = DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;

    let status = metadata
        .as_ref()
        .and_then(|m| m.submitted_chunks.get(&chunk_index))
        .map(|&c| c.status)
        .unwrap_or_default();

    Ok(DealingChunkStatusResponse {
        epoch_id,
        dealer,
        dealing_index,
        chunk_index,
        status,
    })
}

/// Get the particular chunk of the dealing.
pub fn query_dealing_chunk(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
    chunk_index: ChunkIndex,
) -> StdResult<DealingChunkResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let chunk = StoredDealing::read(deps.storage, epoch_id, &dealer, dealing_index, chunk_index);

    Ok(DealingChunkResponse {
        epoch_id,
        dealer,
        dealing_index,
        chunk_index,
        chunk,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::fixtures::{dealing_bytes_fixture, partial_dealing_fixture};
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::{Addr, DepsMut};
    use nym_coconut_dkg_common::dealing::{DealingChunkInfo, PartialContractDealing};

    #[allow(unused)]
    fn fill_dealings(
        deps: DepsMut<'_>,
        epoch: EpochId,
        dealers: usize,
        key_size: u32,
        chunks: u16,
    ) {
        for i in 0..dealers {
            let dealer = Addr::unchecked(format!("dealer{i}"));
            for dealing_index in 0..key_size {
                let data = dealing_bytes_fixture();
                let chunks = data.0.chunks(data.len() / chunks as usize);

                let mut chunk_infos = Vec::new();
                for (chunk_index, chunk) in chunks.enumerate() {
                    chunk_infos.push(DealingChunkInfo { size: chunk.len() });
                    StoredDealing::save(
                        deps.storage,
                        epoch,
                        &dealer,
                        PartialContractDealing {
                            dealing_index,
                            chunk_index: chunk_index as ChunkIndex,
                            data: chunk.into(),
                        },
                    )
                }
            }
        }
    }

    #[test]
    fn test_query_dealing_chunk() {
        let mut deps = init_contract();

        let bad_address = "FOOMP".to_string();
        assert!(query_dealing_chunk(deps.as_ref(), 0, bad_address, 0, 0).is_err());

        let empty = query_dealing_chunk(deps.as_ref(), 0, "foo".to_string(), 0, 0).unwrap();
        assert_eq!(empty.epoch_id, 0);
        assert_eq!(empty.dealing_index, 0);
        assert_eq!(empty.chunk_index, 0);
        assert_eq!(empty.dealer, Addr::unchecked("foo"));
        assert!(empty.chunk.is_none());

        // insert the dealing chunk
        let dealing = partial_dealing_fixture();
        StoredDealing::save(
            deps.as_mut().storage,
            0,
            &Addr::unchecked("foo"),
            dealing.clone(),
        );

        let retrieved = query_dealing_chunk(deps.as_ref(), 0, "foo".to_string(), 0, 0).unwrap();
        assert_eq!(retrieved.epoch_id, 0);
        assert_eq!(retrieved.dealing_index, dealing.dealing_index);
        assert_eq!(retrieved.chunk_index, dealing.chunk_index);
        assert_eq!(retrieved.dealer, Addr::unchecked("foo"));
        assert_eq!(retrieved.chunk.unwrap(), dealing.data);
    }

    #[test]
    fn test_query_dealing_status() {
        let deps = init_contract();

        let bad_address = "FOOMP".to_string();
        assert!(query_dealing_status(deps.as_ref(), 0, bad_address, 0).is_err());

        let empty = query_dealing_status(deps.as_ref(), 0, "foo".to_string(), 0).unwrap();
        assert_eq!(empty.epoch_id, 0);
        assert_eq!(empty.dealing_index, 0);
        assert_eq!(empty.dealer, Addr::unchecked("foo"));
        assert!(!empty.status.fully_submitted);
        assert!(!empty.status.has_metadata);
        assert!(empty.status.chunk_submission_status.is_empty());

        // insert the metadata
        //

        // // insert the dealing
        // let dealing = partial_dealing_fixture();
        // StoredDealing::save(
        //     deps.as_mut().storage,
        //     0,
        //     &Addr::unchecked("foo"),
        //     dealing.clone(),
        // );
        //
        // let retrieved = query_dealing_status(deps.as_ref(), 0, "foo".to_string(), 0).unwrap();
        // assert_eq!(retrieved.epoch_id, 0);
        // assert_eq!(retrieved.dealing_index, dealing.dealing_index);
        // assert_eq!(retrieved.dealer, Addr::unchecked("foo"));
        // assert!(retrieved.dealing_submitted)
    }
}
