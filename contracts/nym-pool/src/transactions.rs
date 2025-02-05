// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_POOL_STORAGE;
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response};
use nym_pool_contract_common::{Allowance, NymPoolContractError, TransferRecipient};

pub fn try_update_contract_admin(
    mut deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, NymPoolContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = NYM_POOL_STORAGE.contract_admin.execute_update_admin(
        deps.branch(),
        info,
        Some(new_admin.clone()),
    )?;

    Ok(res)
}

pub fn try_grant_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    grantee: String,
    allowance: Allowance,
) -> Result<Response, NymPoolContractError> {
    let grantee = deps.api.addr_validate(&grantee)?;

    NYM_POOL_STORAGE.add_grant(deps, &env, &info.sender, grantee, allowance)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_revoke_grant(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    grantee: String,
) -> Result<Response, NymPoolContractError> {
    let grantee = deps.api.addr_validate(&grantee)?;

    NYM_POOL_STORAGE.revoke_grant(deps, grantee, info.sender)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_use_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    recipients: Vec<TransferRecipient>,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

pub fn try_withdraw_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

pub fn try_lock_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

pub fn try_unlock_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

pub fn try_use_locked_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    recipients: Vec<TransferRecipient>,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

pub fn try_withdraw_locked_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod updating_contract_admin {
        use super::*;
        use crate::testing::TestSetup;
        use cw_controllers::AdminError;
        use nym_pool_contract_common::ExecuteMsg;

        #[test]
        fn can_only_be_performed_by_current_admin() -> anyhow::Result<()> {
            let mut test = TestSetup::init();

            let random_acc = test.generate_account();
            let new_admin = test.generate_account();
            let res = test
                .execute_raw(
                    random_acc,
                    ExecuteMsg::UpdateAdmin {
                        admin: new_admin.to_string(),
                    },
                )
                .unwrap_err();

            assert_eq!(res, NymPoolContractError::Admin(AdminError::NotAdmin {}));

            let actual_admin = test.admin_unchecked();
            let res = test.execute_raw(
                actual_admin.clone(),
                ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            );
            assert!(res.is_ok());

            let updated_admin = test.admin_unchecked();
            assert_eq!(new_admin, updated_admin);

            Ok(())
        }

        #[test]
        fn requires_providing_valid_address() -> anyhow::Result<()> {
            let mut test = TestSetup::init();

            let bad_account = "definitely-not-valid-account";
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateAdmin {
                    admin: bad_account.to_string(),
                },
            );

            assert!(res.is_err());

            let empty_account = "";
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateAdmin {
                    admin: empty_account.to_string(),
                },
            );

            assert!(res.is_err());

            Ok(())
        }
    }

    #[cfg(test)]
    mod granting_allowance {
        use super::*;
        use crate::testing::TestSetup;
        use cosmwasm_std::StdError;
        use nym_pool_contract_common::BasicAllowance;

        #[test]
        fn requires_providing_valid_grantee_address() -> anyhow::Result<()> {
            let mut test = TestSetup::init();

            let env = test.env();
            let admin = test.admin_msg();
            let dummy_grant = Allowance::Basic(BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: None,
            });

            assert!(matches!(
                try_grant_allowance(
                    test.deps_mut(),
                    env.clone(),
                    admin.clone(),
                    "not-a-valid-address".to_string(),
                    dummy_grant.clone()
                )
                .unwrap_err(),
                NymPoolContractError::StdErr(StdError::GenericErr { msg, .. }) if msg == "Error decoding bech32"
            ));

            let valid_address = test.generate_account();
            assert!(try_grant_allowance(
                test.deps_mut(),
                env.clone(),
                admin.clone(),
                valid_address.to_string(),
                dummy_grant
            )
            .is_ok());

            Ok(())
        }
    }

    #[cfg(test)]
    mod revoking_allowance {
        use super::*;
        use crate::testing::TestSetup;
        use cosmwasm_std::StdError;

        #[test]
        fn requires_providing_valid_grantee_address() -> anyhow::Result<()> {
            let mut test = TestSetup::init();

            let env = test.env();
            let admin = test.admin_msg();
            let grant = test.add_dummy_grant();

            assert!(matches!(
                try_revoke_grant(
                    test.deps_mut(),
                    env.clone(),
                    admin.clone(),
                    "not-a-valid-address".to_string()
                )
                .unwrap_err(),
                NymPoolContractError::StdErr(StdError::GenericErr { msg, .. }) if msg == "Error decoding bech32"
            ));

            // use a valid address
            // note the different error
            let valid_address = test.generate_account();
            assert_eq!(
                try_revoke_grant(
                    test.deps_mut(),
                    env.clone(),
                    admin.clone(),
                    valid_address.to_string()
                )
                .unwrap_err(),
                NymPoolContractError::GrantNotFound {
                    grantee: valid_address.to_string()
                }
            );

            // for sanity’s sake check with an existing grant
            assert!(try_revoke_grant(
                test.deps_mut(),
                env.clone(),
                admin.clone(),
                grant.grantee.to_string()
            )
            .is_ok());

            Ok(())
        }
    }
}
