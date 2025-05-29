// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::Deps;
use cw_controllers::AdminResponse;
use nym_performance_contract_common::NymPerformanceContractError;

pub fn query_admin(deps: Deps) -> Result<AdminResponse, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod admin_query {
        use super::*;
        use crate::testing::{init_contract_tester, PerformanceContractTesterExt};
        use nym_performance_contract_common::ExecuteMsg;

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
