// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Order, StdError, StdResult};
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::IdentityKey;
use mixnet_contract::PagedAllDelegationsResponse;
use mixnet_contract::UnpackedDelegation;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub(crate) fn get_all_delegations_paged<T>(
    bucket: &ReadonlyBucket<T>,
    start_after: &Option<Vec<u8>>,
    limit: usize,
) -> StdResult<PagedAllDelegationsResponse<T>>
where
    T: Serialize + DeserializeOwned,
{
    let delegations = bucket
        .range(start_after.as_deref(), None, Order::Ascending)
        .filter(|res| res.is_ok())
        .take(limit)
        .map(|res| {
            res.map(|entry| {
                let (owner, identity) = extract_identity_and_owner(entry.0).expect("Invalid node identity or address used as key in bucket. The storage is corrupted!");
                UnpackedDelegation::new(owner, identity, entry.1)
            })
        })
        .collect::<StdResult<Vec<UnpackedDelegation<T>>>>()?;

    let start_next_after = if let Some(Ok(last)) = bucket
        .range(start_after.as_deref(), None, Order::Ascending)
        .filter(|res| res.is_ok())
        .take(limit)
        .last()
    {
        Some(last.0)
    } else {
        None
    };

    Ok(PagedAllDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

// Extracts the node identity and owner of a delegation from the bytes used as
// key in the delegation buckets.
fn extract_identity_and_owner(bytes: Vec<u8>) -> StdResult<(Addr, IdentityKey)> {
    // cosmwasm bucket internal value
    const NAMESPACE_LENGTH: usize = 2;

    if bytes.len() < NAMESPACE_LENGTH {
        return Err(StdError::parse_err(
            "mixnet_contract::types::IdentityKey",
            "Invalid type",
        ));
    }
    let identity_size = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
    let identity_bytes: Vec<u8> = bytes
        .iter()
        .skip(NAMESPACE_LENGTH)
        .take(identity_size)
        .copied()
        .collect();
    let identity = IdentityKey::from_utf8(identity_bytes)
        .map_err(|_| StdError::parse_err("mixnet_contract::types::IdentityKey", "Invalid type"))?;
    let owner_bytes: Vec<u8> = bytes
        .iter()
        .skip(NAMESPACE_LENGTH + identity_size)
        .copied()
        .collect();
    let owner = Addr::unchecked(
        String::from_utf8(owner_bytes)
            .map_err(|_| StdError::parse_err("cosmwasm_std::addresses::Addr", "Invalid type"))?,
    );

    Ok((owner, identity))
}

#[cfg(test)]
pub(crate) const OLD_DELEGATIONS_CHUNK_SIZE: usize = 500;

#[cfg(test)]
pub struct Delegations<'a, T: Clone + Serialize + DeserializeOwned> {
    delegations_bucket: ReadonlyBucket<'a, T>,
    curr_delegations: Vec<UnpackedDelegation<T>>,
    curr_index: usize,
    start_after: Option<Vec<u8>>,
    last_page: bool,
}

#[cfg(test)]
impl<'a, T: Clone + Serialize + DeserializeOwned> Delegations<'a, T> {
    pub fn new(delegations_bucket: ReadonlyBucket<'a, T>) -> Self {
        Delegations {
            delegations_bucket,
            curr_delegations: vec![],
            curr_index: OLD_DELEGATIONS_CHUNK_SIZE,
            start_after: None,
            last_page: false,
        }
    }
}

#[cfg(test)]
impl<'a, T: Clone + Serialize + DeserializeOwned> Iterator for Delegations<'a, T> {
    type Item = UnpackedDelegation<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_index == OLD_DELEGATIONS_CHUNK_SIZE && !self.last_page {
            self.start_after = self.start_after.clone().map(|mut v: Vec<u8>| {
                v.push(0);
                v
            });
            let delegations_paged = get_all_delegations_paged(
                &self.delegations_bucket,
                &self.start_after,
                OLD_DELEGATIONS_CHUNK_SIZE,
            )
            .ok()?;
            self.curr_delegations = delegations_paged.delegations;
            self.curr_index = 0;
            self.start_after = delegations_paged.start_next_after;
            if self.start_after.is_none() {
                self.last_page = true;
            }
        }
        if self.curr_index < self.curr_delegations.len() {
            let ret = self.curr_delegations[self.curr_index].clone();
            self.curr_index += 1;
            Some(ret)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnodes::delegation_queries::tests::store_n_mix_delegations;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::identity_and_owner_to_bytes;
    use cosmwasm_std::testing::mock_dependencies;
    use mixnet_contract::RawDelegationData;

    #[test]
    fn identity_and_owner_serialization() {
        let identity: IdentityKey = "gateway".into();
        let owner = Addr::unchecked("bob");
        assert_eq!(
            vec![0, 7, 103, 97, 116, 101, 119, 97, 121, 98, 111, 98],
            identity_and_owner_to_bytes(&identity, &owner)
        );
    }

    #[test]
    fn identity_and_owner_deserialization() {
        assert!(extract_identity_and_owner(vec![]).is_err());
        assert!(extract_identity_and_owner(vec![0]).is_err());
        let (owner, identity) = extract_identity_and_owner(vec![
            0, 7, 109, 105, 120, 110, 111, 100, 101, 97, 108, 105, 99, 101,
        ])
        .unwrap();
        assert_eq!(owner, "alice");
        assert_eq!(identity, "mixnode");
    }

    #[test]
    fn delegations_iterator() {
        let mut deps = test_helpers::init_contract();
        let node_identity: IdentityKey = "foo".into();

        store_n_mix_delegations(
            2 * OLD_DELEGATIONS_CHUNK_SIZE as u32,
            &mut deps.storage,
            &node_identity,
        );
        let mix_bucket =
            mixnodes_storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage);
        let mut delegations = Delegations::new(mix_bucket);
        assert!(delegations.curr_delegations.is_empty());
        assert_eq!(delegations.curr_index, OLD_DELEGATIONS_CHUNK_SIZE);
        delegations.next().unwrap();
        assert_eq!(
            delegations.curr_delegations.len(),
            OLD_DELEGATIONS_CHUNK_SIZE
        );
        assert_eq!(delegations.curr_index, 1);
        for _ in 0..OLD_DELEGATIONS_CHUNK_SIZE {
            delegations.next().unwrap();
        }
        assert_eq!(
            delegations.curr_delegations.len(),
            OLD_DELEGATIONS_CHUNK_SIZE
        );
        assert_eq!(delegations.curr_index, 1);
        for _ in 0..OLD_DELEGATIONS_CHUNK_SIZE - 1 {
            delegations.next().unwrap();
        }
        assert!(delegations.next().is_none());
    }

    #[test]
    fn all_mix_delegations() {
        let mut deps = mock_dependencies(&[]);
        let node_identity1: IdentityKey = "foo1".into();
        let delegation_owner1 = Addr::unchecked("bar1");
        let node_identity2: IdentityKey = "foo2".into();
        let delegation_owner2 = Addr::unchecked("bar2");
        let raw_delegation = RawDelegationData::new(1000u128.into(), 42);
        let mut start_after = None;

        mixnodes_storage::mix_delegations(&mut deps.storage, &node_identity1)
            .save(delegation_owner1.as_bytes(), &raw_delegation)
            .unwrap();

        let bucket = mixnodes_storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage);
        let response =
            get_all_delegations_paged::<RawDelegationData>(&bucket, &start_after, 10).unwrap();
        start_after = response.start_next_after;
        let delegations = response.delegations;
        assert_eq!(delegations.len(), 1);
        assert_eq!(
            delegations[0],
            UnpackedDelegation::new(
                delegation_owner1.clone(),
                node_identity1.clone(),
                raw_delegation.clone()
            )
        );

        mixnodes_storage::mix_delegations(&mut deps.storage, &node_identity2)
            .save(delegation_owner2.as_bytes(), &raw_delegation)
            .unwrap();

        let bucket = mixnodes_storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage);
        let response =
            get_all_delegations_paged::<RawDelegationData>(&bucket, &start_after, 10).unwrap();
        start_after = response.start_next_after;
        let delegations = response.delegations;
        assert_eq!(delegations.len(), 2);
        assert_eq!(
            delegations[1],
            UnpackedDelegation::new(
                delegation_owner2.clone(),
                node_identity2.clone(),
                raw_delegation.clone()
            )
        );

        mixnodes_storage::mix_delegations(&mut deps.storage, &node_identity1)
            .remove(delegation_owner1.as_bytes());

        let bucket = mixnodes_storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage);
        let response =
            get_all_delegations_paged::<RawDelegationData>(&bucket, &start_after, 10).unwrap();
        let delegations = response.delegations;
        assert_eq!(delegations.len(), 1);
        assert_eq!(
            delegations[0],
            UnpackedDelegation::new(delegation_owner2, node_identity2, raw_delegation.clone()),
        );
    }
}
