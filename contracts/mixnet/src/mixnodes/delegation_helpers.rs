use cosmwasm_std::Addr;
use cosmwasm_std::Order;
use cosmwasm_std::StdError;
use cosmwasm_std::StdResult;
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
mod tests {
    use super::*;
    use crate::mixnodes::delegation_queries::tests::store_n_mix_delegations;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::support::tests::helpers;
    use cosmwasm_std::testing::mock_dependencies;
    use mixnet_contract::RawDelegationData;

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
}
