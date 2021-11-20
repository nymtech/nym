// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnodes::delegation_helpers::get_all_delegations_paged;
use crate::mixnodes::delegation_transactions::OLD_DELEGATIONS_CHUNK_SIZE;
use cosmwasm_std::{Order, StdError, StdResult};
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::{Addr, IdentityKey, PagedAllDelegationsResponse, UnpackedDelegation};
use serde::de::DeserializeOwned;
use serde::Serialize;

// currently not used outside tests
#[cfg(test)]
// Converts the node identity and owner of a delegation into the bytes used as
// key in the delegation buckets.
pub(crate) fn identity_and_owner_to_bytes(identity: &str, owner: &Addr) -> Vec<u8> {
    let mut bytes = u16::to_be_bytes(identity.len() as u16).to_vec();
    bytes.append(&mut identity.as_bytes().to_vec());
    bytes.append(&mut owner.as_bytes().to_vec());

    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnodes::delegation_queries::tests::store_n_mix_delegations;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::support::tests::helpers;
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
}
