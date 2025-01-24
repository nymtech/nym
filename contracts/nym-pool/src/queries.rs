// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_POOL_STORAGE;
use cosmwasm_std::{Deps, StdResult};
use cw_controllers::AdminResponse;

pub fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    NYM_POOL_STORAGE.contract_admin.query_admin(deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod admin_query {
        use super::*;
        use crate::testing::TestSetup;
        use nym_pool_contract_common::ExecuteMsg;

        #[test]
        fn returns_current_admin() -> anyhow::Result<()> {
            let mut test = TestSetup::init();

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
