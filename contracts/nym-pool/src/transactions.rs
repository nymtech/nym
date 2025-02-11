// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::validate_usage_coin;
use crate::storage::NYM_POOL_STORAGE;
use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use nym_pool_contract_common::{Allowance, NymPoolContractError, TransferRecipient};

pub fn try_update_contract_admin(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    new_admin: String,
    update_granter_set: Option<bool>,
) -> Result<Response, NymPoolContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;
    let old_admin = info.sender.clone();

    if let Some(true) = update_granter_set {
        // remove current/old admin from the granter set, if present
        NYM_POOL_STORAGE
            .granters
            .remove(deps.storage, old_admin.clone());

        // insert new admin into the granter set
        NYM_POOL_STORAGE.add_new_granter(deps.branch(), &env, &old_admin, &new_admin)?;
    }

    let res = NYM_POOL_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?;

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

    NYM_POOL_STORAGE.insert_new_grant(deps, &env, &info.sender, &grantee, allowance)?;

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

    NYM_POOL_STORAGE.revoke_grant(deps, &grantee, &info.sender)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_use_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    recipients: Vec<TransferRecipient>,
) -> Result<Response, NymPoolContractError> {
    let denom = NYM_POOL_STORAGE.pool_denomination.load(deps.storage)?;

    let mut amount = Uint128::zero();
    let mut messages = Vec::new();
    for recipient in recipients {
        validate_usage_coin(deps.storage, &recipient.amount)?;

        amount += recipient.amount.amount;
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.recipient,
            amount: vec![recipient.amount],
        }))
    }

    NYM_POOL_STORAGE.try_spend_part_of_grant(deps, &env, &info.sender, &Coin { amount, denom })?;

    // TODO: emit events
    Ok(Response::new().add_messages(messages))
}

pub fn try_withdraw_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    validate_usage_coin(deps.storage, &amount)?;
    NYM_POOL_STORAGE.try_spend_part_of_grant(deps, &env, &info.sender, &amount)?;

    // TODO: emit events
    // TODO2: after migrating common to cw2.2 use `send_tokens` from `ResponseExt` trait
    Ok(Response::new().add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![amount],
    })))
}

pub fn try_lock_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    NYM_POOL_STORAGE.lock_part_of_allowance(deps, &env, &info.sender, amount)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_unlock_allowance(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    NYM_POOL_STORAGE.unlock_part_of_allowance(deps, &info.sender, &amount)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_use_locked_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    recipients: Vec<TransferRecipient>,
) -> Result<Response, NymPoolContractError> {
    let mut amount = Uint128::zero();
    let mut messages = Vec::new();
    for recipient in recipients {
        validate_usage_coin(deps.storage, &recipient.amount)?;

        amount += recipient.amount.amount;
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.recipient,
            amount: vec![recipient.amount],
        }))
    }

    // if the grant has already expired, locked coins can no longer be used,
    // ideally, they'd be immediately unlocked here, but we need to revert the transaction
    let grant = NYM_POOL_STORAGE.load_grant(deps.as_ref(), &info.sender)?;
    if grant.allowance.expired(&env) {
        return Err(NymPoolContractError::GrantExpired);
    }

    let denom = NYM_POOL_STORAGE.pool_denomination.load(deps.storage)?;

    // we remove those coins from the locked pool before transferring them to the specified account
    NYM_POOL_STORAGE.unlock_part_of_allowance(deps, &info.sender, &Coin { amount, denom })?;

    // TODO: emit events
    Ok(Response::new().add_messages(messages))
}

pub fn try_withdraw_locked_allowance(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, NymPoolContractError> {
    // if the grant has already expired, locked coins can no longer be used,
    // ideally, they'd be immediately unlocked here, but we need to revert the transaction
    let grant = NYM_POOL_STORAGE.load_grant(deps.as_ref(), &info.sender)?;
    if grant.allowance.expired(&env) {
        return Err(NymPoolContractError::GrantExpired);
    }

    // we remove those coins from the locked pool before transferring them to the specified account
    NYM_POOL_STORAGE.unlock_part_of_allowance(deps, &info.sender, &amount)?;

    // TODO: emit events
    // TODO2: after migrating common to cw2.2 use `send_tokens` from `ResponseExt` trait
    Ok(Response::new().add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![amount],
    })))
}

// can be called by anyone, because expired grants are unusable anyway
pub fn try_remove_expired(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    grantee: String,
) -> Result<Response, NymPoolContractError> {
    let grantee = deps.api.addr_validate(&grantee)?;
    let grant = NYM_POOL_STORAGE.load_grant(deps.as_ref(), &grantee)?;

    if !grant.allowance.expired(&env) {
        return Err(NymPoolContractError::GrantNotExpired);
    }

    NYM_POOL_STORAGE.remove_grant(deps, &grantee)?;

    // TODO: emit events
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod updating_contract_admin {
        use super::*;
        use crate::testing::TestSetup;
        use cosmwasm_std::{Deps, Order};
        use cw_controllers::AdminError;
        use nym_pool_contract_common::{ExecuteMsg, GranterAddress, GranterInformation};
        use std::collections::HashMap;

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
                        update_granter_set: None,
                    },
                )
                .unwrap_err();

            assert_eq!(res, NymPoolContractError::Admin(AdminError::NotAdmin {}));

            let actual_admin = test.admin_unchecked();
            let res = test.execute_raw(
                actual_admin.clone(),
                ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                    update_granter_set: None,
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
                    update_granter_set: None,
                },
            );

            assert!(res.is_err());

            let empty_account = "";
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateAdmin {
                    admin: empty_account.to_string(),
                    update_granter_set: None,
                },
            );

            assert!(res.is_err());

            Ok(())
        }

        #[test]
        fn updates_granter_set_if_specified() {
            fn granters(deps: Deps) -> HashMap<GranterAddress, GranterInformation> {
                NYM_POOL_STORAGE
                    .granters
                    .range(deps.storage, None, None, Order::Ascending)
                    .map(|res| res.unwrap())
                    .collect()
            }

            let mut test = TestSetup::init();
            let current_admin = test.admin_unchecked();
            let new_admin = test.generate_account();

            let old_granters = granters(test.deps());

            // no change to the granter set
            let res = test.execute_raw(
                current_admin.clone(),
                ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                    update_granter_set: Some(false),
                },
            );
            assert!(res.is_ok());
            let new_granters = granters(test.deps());
            assert_eq!(old_granters, new_granters);

            //
            //
            //

            let mut test = TestSetup::init();
            let current_admin = test.admin_unchecked();
            let new_admin = test.generate_account();
            let old_granters = granters(test.deps());

            let res = test.execute_raw(
                current_admin.clone(),
                ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                    update_granter_set: Some(true),
                },
            );
            assert!(res.is_ok());
            let new_granters = granters(test.deps());
            assert_ne!(old_granters, new_granters);
            assert!(old_granters.contains_key(&current_admin));
            assert!(new_granters.contains_key(&new_admin));
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
            let dummy_grant = Allowance::Basic(BasicAllowance::unlimited());

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
