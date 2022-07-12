// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_bandwidth_contract_common::spend_credential::{
    PagedSpendCredentialResponse, SpendCredential, SpendCredentialResponse,
};
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::storage::{self, SPEND_CREDENTIAL_PAGE_DEFAULT_LIMIT, SPEND_CREDENTIAL_PAGE_MAX_LIMIT};

pub(crate) fn query_spent_credentials_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedSpendCredentialResponse> {
    let limit = limit
        .unwrap_or(SPEND_CREDENTIAL_PAGE_DEFAULT_LIMIT)
        .min(SPEND_CREDENTIAL_PAGE_MAX_LIMIT) as usize;

    let start = start_after.as_deref().map(Bound::exclusive);

    let nodes = storage::spent_credentials()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<SpendCredential>>>()?;

    let start_next_after = nodes
        .last()
        .map(|spend_credential| spend_credential.blinded_serial_number().to_string());

    Ok(PagedSpendCredentialResponse::new(
        nodes,
        limit,
        start_next_after,
    ))
}

pub(crate) fn query_spent_credential(
    deps: Deps<'_>,
    blinded_serial_number: String,
) -> StdResult<SpendCredentialResponse> {
    let spend_credential =
        storage::spent_credentials().may_load(deps.storage, &blinded_serial_number)?;
    Ok(SpendCredentialResponse::new(spend_credential))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::fixtures::spend_credential_data_fixture;
    use crate::support::tests::helpers::init_contract;
    use crate::transactions::spend_credential;
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn spent_credentials_empty_on_init() {
        let deps = init_contract();
        let response = query_spent_credentials_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.spend_credentials.len());
    }

    #[test]
    fn spent_credentials_paged_retrieval_obeys_limits() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        let limit = 2;
        for n in 0..1000 {
            let data = spend_credential_data_fixture(&format!("blinded_serial_number{}", n));
            spend_credential(deps.as_mut(), env.clone(), info.clone(), data).unwrap();
        }

        let page1 =
            query_spent_credentials_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.spend_credentials.len() as u32);
    }

    #[test]
    fn spent_credentials_paged_retrieval_has_default_limit() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        for n in 0..1000 {
            let data = spend_credential_data_fixture(&format!("blinded_serial_number{}", n));
            spend_credential(deps.as_mut(), env.clone(), info.clone(), data).unwrap();
        }

        // query without explicitly setting a limit
        let page1 = query_spent_credentials_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(
            SPEND_CREDENTIAL_PAGE_DEFAULT_LIMIT,
            page1.spend_credentials.len() as u32
        );
    }

    #[test]
    fn spent_credentials_paged_retrieval_has_max_limit() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        for n in 0..1000 {
            let data = spend_credential_data_fixture(&format!("blinded_serial_number{}", n));
            spend_credential(deps.as_mut(), env.clone(), info.clone(), data).unwrap();
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * SPEND_CREDENTIAL_PAGE_MAX_LIMIT;
        let page1 =
            query_spent_credentials_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = SPEND_CREDENTIAL_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.spend_credentials.len() as u32);
    }

    #[test]
    fn spent_credentials_pagination_works() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        let data = spend_credential_data_fixture("blinded_serial_number1");
        spend_credential(deps.as_mut(), env.clone(), info.clone(), data).unwrap();

        let per_page = 2;
        let page1 =
            query_spent_credentials_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.spend_credentials.len());

        // save another
        let data = spend_credential_data_fixture("blinded_serial_number2");
        spend_credential(deps.as_mut(), env.clone(), info.clone(), data).unwrap();

        // page1 should have 2 results on it
        let page1 =
            query_spent_credentials_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.spend_credentials.len());

        let data = spend_credential_data_fixture("blinded_serial_number3");
        spend_credential(deps.as_mut(), env.clone(), info.clone(), data).unwrap();

        // page1 still has 2 results
        let page1 =
            query_spent_credentials_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.spend_credentials.len());

        // retrieving the next page should start after the last key on this page
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_spent_credentials_paged(
            deps.as_ref(),
            Option::from(start_after.clone()),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.spend_credentials.len());

        let data = spend_credential_data_fixture("blinded_serial_number4");
        spend_credential(deps.as_mut(), env, info, data).unwrap();

        let page2 = query_spent_credentials_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.spend_credentials.len());
    }
}
