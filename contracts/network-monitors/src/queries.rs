// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{retrieval_limits, NETWORK_MONITORS_CONTRACT_STORAGE};
use cosmwasm_std::{Deps, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_network_monitors_contract_common::{
    AuthorisedNetworkMonitorOrchestratorsResponse, AuthorisedNetworkMonitorsPagedResponse,
    NetworkMonitorsContractError,
};
use std::net::IpAddr;

pub fn query_admin(deps: Deps) -> Result<AdminResponse, NetworkMonitorsContractError> {
    NETWORK_MONITORS_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

// no need for pagination as we don't expect even a double digit of those
pub fn query_network_monitor_orchestrators(
    deps: Deps,
) -> Result<AuthorisedNetworkMonitorOrchestratorsResponse, NetworkMonitorsContractError> {
    let authorised = NETWORK_MONITORS_CONTRACT_STORAGE
        .authorised_orchestrators
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|record| record.map(|(_, details)| details))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AuthorisedNetworkMonitorOrchestratorsResponse { authorised })
}

pub fn query_network_monitor_agents(
    deps: Deps,
    start_after: Option<IpAddr>,
    limit: Option<u32>,
) -> Result<AuthorisedNetworkMonitorsPagedResponse, NetworkMonitorsContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::AGENTS_DEFAULT_LIMIT)
        .min(retrieval_limits::AGENTS_MAX_LIMIT) as usize;

    let start = start_after.map(|addr| Bound::exclusive(addr.to_string()));

    let authorised = NETWORK_MONITORS_CONTRACT_STORAGE
        .authorised_agents
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|record| record.map(|(_, details)| details))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = authorised.last().map(|last| last.address);

    Ok(AuthorisedNetworkMonitorsPagedResponse {
        authorised,
        start_next_after,
    })
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    mod admin_query {
        use crate::queries::query_admin;
        use crate::testing::init_contract_tester;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_network_monitors_contract_common::ExecuteMsg;

        #[test]
        fn returns_current_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let initial_admin = test.admin_unchecked();

            // initial
            let res = query_admin(test.deps())?;
            assert_eq!(res.admin, Some(initial_admin.to_string()));

            let new_admin = test.generate_account();

            // sanity check
            assert_ne!(initial_admin, new_admin);

            // after update
            test.execute_msg(
                initial_admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            )?;

            let updated_admin = query_admin(test.deps())?;
            assert_eq!(updated_admin.admin, Some(new_admin.to_string()));

            Ok(())
        }
    }
}
