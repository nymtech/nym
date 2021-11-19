// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::transactions::OLD_DELEGATIONS_CHUNK_SIZE;
use cosmwasm_std::{Decimal, Order, StdError, StdResult, Uint128};
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::{Addr, IdentityKey, PagedAllDelegationsResponse, UnpackedDelegation};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::Sub;

// for time being completely ignore concept of a leap year and assume each year is exactly 365 days
// i.e. 8760 hours
const HOURS_IN_YEAR: u128 = 8760;

const DECIMAL_FRACTIONAL: Uint128 = Uint128(1_000_000_000_000_000_000u128);

// cosmwasm bucket internal value
const NAMESPACE_LENGTH: usize = 2;

pub fn decimal_to_uint128(value: Decimal) -> Uint128 {
    value * DECIMAL_FRACTIONAL
}

pub fn uint128_to_decimal(value: Uint128) -> Decimal {
    Decimal::from_ratio(value, DECIMAL_FRACTIONAL)
}

pub(crate) fn calculate_epoch_reward_rate(
    epoch_length: u32,
    annual_reward_rate: Decimal,
) -> Decimal {
    // this is more of a sanity check as the contract does not allow setting annual reward rates
    // to be lower than 1.
    debug_assert!(annual_reward_rate >= Decimal::one());

    // converts reward rate, like 1.25 into the expected gain, like 0.25
    let annual_reward = annual_reward_rate.sub(Decimal::one());
    // do a simple cross-multiplication:
    // `annual_reward`  -    `HOURS_IN_YEAR`
    //          x       -    `epoch_length`
    //
    // x = `annual_reward` * `epoch_length` / `HOURS_IN_YEAR`

    // converts reward, like 0.25 into 250000000000000000
    let annual_reward_uint128 = decimal_to_uint128(annual_reward);

    // calculates `annual_reward_uint128` * `epoch_length` / `HOURS_IN_YEAR`
    let epoch_reward_uint128 = annual_reward_uint128.multiply_ratio(epoch_length, HOURS_IN_YEAR);

    // note: this returns a % reward, like 0.05 rather than reward rate (like 1.05)
    uint128_to_decimal(epoch_reward_uint128)
}

pub(crate) fn scale_reward_by_uptime(
    reward: Decimal,
    uptime: u32,
) -> Result<Decimal, ContractError> {
    if uptime > 100 {
        return Err(ContractError::UnexpectedUptime);
    }
    let uptime_ratio = Decimal::from_ratio(uptime, 100u128);
    // if we do not convert into a more precise representation, we might end up with, for example,
    // reward 0.05 and uptime of 50% which would produce 0.50 * 0.05 = 0 (because of u128 representation)
    // and also the above would be impossible to compute as Mul<Decimal> for Decimal is not implemented
    //
    // but with the intermediate conversion, we would have
    // 0.50 * 50_000_000_000_000_000 = 25_000_000_000_000_000
    // which converted back would give us the proper 0.025
    let uptime_ratio_u128 = decimal_to_uint128(uptime_ratio);
    let scaled = reward * uptime_ratio_u128;
    Ok(uint128_to_decimal(scaled))
}

// Extracts the node identity and owner of a delegation from the bytes used as
// key in the delegation buckets.
fn extract_identity_and_owner(bytes: Vec<u8>) -> StdResult<(Addr, IdentityKey)> {
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

pub struct Delegations<'a, T: Clone + Serialize + DeserializeOwned> {
    delegations_bucket: ReadonlyBucket<'a, T>,
    curr_delegations: Vec<UnpackedDelegation<T>>,
    curr_index: usize,
    start_after: Option<Vec<u8>>,
    last_page: bool,
}

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
    use crate::queries::tests::store_n_mix_delegations;
    use crate::storage::{all_mix_delegations_read, mix_delegations};
    use crate::support::tests::helpers;
    use cosmwasm_std::testing::mock_dependencies;
    use mixnet_contract::RawDelegationData;
    use std::str::FromStr;

    #[test]
    fn delegations_iterator() {
        let mut deps = helpers::init_contract();
        let node_identity: IdentityKey = "foo".into();

        store_n_mix_delegations(
            2 * OLD_DELEGATIONS_CHUNK_SIZE as u32,
            &mut deps.storage,
            &node_identity,
        );
        let mix_bucket = all_mix_delegations_read::<RawDelegationData>(&deps.storage);
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
    fn calculating_epoch_reward_rate() {
        // 1.10
        let annual_reward_rate = Decimal::from_ratio(110u128, 100u128);

        // if the epoch is (for some reason) exactly one year,
        // the reward rate should be unchanged
        let per_epoch_rate = calculate_epoch_reward_rate(HOURS_IN_YEAR as u32, annual_reward_rate);
        // 0.10
        let expected = annual_reward_rate.sub(Decimal::one());
        assert_eq!(expected, per_epoch_rate);

        // 24 hours
        let per_epoch_rate = calculate_epoch_reward_rate(24, annual_reward_rate);
        // 0.1 / 365
        let expected = Decimal::from_ratio(1u128, 3650u128);
        assert_eq!(expected, per_epoch_rate);

        let expected_per_epoch_rate_excel = Decimal::from_str("0.000273972602739726").unwrap();
        assert_eq!(expected_per_epoch_rate_excel, per_epoch_rate);

        // 1 hour
        let per_epoch_rate = calculate_epoch_reward_rate(1, annual_reward_rate);
        // 0.1 / 8760
        let expected = Decimal::from_ratio(1u128, 87600u128);
        assert_eq!(expected, per_epoch_rate);
    }

    #[test]
    fn scaling_reward_by_uptime() {
        // 0.05
        let epoch_reward = Decimal::from_ratio(5u128, 100u128);

        // scaling by 100 does nothing
        let scaled = scale_reward_by_uptime(epoch_reward, 100).unwrap();
        assert_eq!(epoch_reward, scaled);

        // scaling by 0 makes the reward 0
        let scaled = scale_reward_by_uptime(epoch_reward, 0).unwrap();
        assert_eq!(Decimal::zero(), scaled);

        // 50 halves it
        let scaled = scale_reward_by_uptime(epoch_reward, 50).unwrap();
        let expected = Decimal::from_ratio(25u128, 1000u128);
        assert_eq!(expected, scaled);

        // 10 takes 1/10th
        let scaled = scale_reward_by_uptime(epoch_reward, 10).unwrap();
        let expected = Decimal::from_ratio(5u128, 1000u128);
        assert_eq!(expected, scaled);

        // anything larger than 100 returns an error
        assert!(scale_reward_by_uptime(epoch_reward, 101).is_err())
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
    fn identity_and_owner_serialization() {
        let identity: IdentityKey = "gateway".into();
        let owner = Addr::unchecked("bob");
        assert_eq!(
            vec![0, 7, 103, 97, 116, 101, 119, 97, 121, 98, 111, 98],
            identity_and_owner_to_bytes(&identity, &owner)
        );
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

        mix_delegations(&mut deps.storage, &node_identity1)
            .save(delegation_owner1.as_bytes(), &raw_delegation)
            .unwrap();

        let bucket = all_mix_delegations_read::<RawDelegationData>(&deps.storage);
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

        mix_delegations(&mut deps.storage, &node_identity2)
            .save(delegation_owner2.as_bytes(), &raw_delegation)
            .unwrap();

        let bucket = all_mix_delegations_read::<RawDelegationData>(&deps.storage);
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

        mix_delegations(&mut deps.storage, &node_identity1).remove(delegation_owner1.as_bytes());

        let bucket = all_mix_delegations_read::<RawDelegationData>(&deps.storage);
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
