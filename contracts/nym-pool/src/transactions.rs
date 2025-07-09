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

    if recipients.is_empty() {
        return Err(NymPoolContractError::EmptyUsageRequest);
    }

    let mut amount = Uint128::zero();
    let mut messages = Vec::new();
    for recipient in recipients {
        validate_usage_coin(deps.storage, &recipient.amount)?;

        amount += recipient.amount.amount;
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: deps.api.addr_validate(&recipient.recipient)?.to_string(),
            amount: vec![recipient.amount],
        }))
    }

    let available = NYM_POOL_STORAGE.available_tokens(deps.as_ref(), &env)?;
    // even if the contract has sufficient amount of tokens (which would be implicit from BankMsg not failing)
    // the locked ones take priority
    if available.amount < amount {
        return Err(NymPoolContractError::InsufficientTokens {
            available,
            required: Coin { amount, denom },
        });
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

    let available = NYM_POOL_STORAGE.available_tokens(deps.as_ref(), &env)?;

    // even if the contract has sufficient amount of tokens (which would be implicit from BankMsg not failing)
    // the locked ones take priority
    if available.amount < amount.amount {
        return Err(NymPoolContractError::InsufficientTokens {
            available,
            required: amount,
        });
    }

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
            to_address: deps.api.addr_validate(&recipient.recipient)?.to_string(),
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

pub fn try_add_new_granter(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    granter: String,
) -> Result<Response, NymPoolContractError> {
    let granter = deps.api.addr_validate(&granter)?;
    NYM_POOL_STORAGE.add_new_granter(deps, &env, &info.sender, &granter)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_revoke_granter(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    granter: String,
) -> Result<Response, NymPoolContractError> {
    let granter = deps.api.addr_validate(&granter)?;
    NYM_POOL_STORAGE.remove_granter(deps, &info.sender, &granter)?;

    // TODO: emit events
    Ok(Response::new())
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
    use crate::testing::{init_contract_tester, NymPoolContractTesterExt};
    use nym_contracts_common_testing::{AdminExt, ContractOpts, DenomExt, RandExt};
    use nym_pool_contract_common::ExecuteMsg;

    #[cfg(test)]
    mod updating_contract_admin {
        use super::*;
        use cosmwasm_std::{Deps, Order};
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, RandExt};
        use nym_pool_contract_common::{ExecuteMsg, GranterAddress, GranterInformation};
        use std::collections::HashMap;

        #[test]
        fn can_only_be_performed_by_current_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

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
            let mut test = init_contract_tester();

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

            let mut test = init_contract_tester();
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

            let mut test = init_contract_tester();
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
        use cosmwasm_std::StdError;
        use nym_contracts_common_testing::{AdminExt, RandExt};
        use nym_pool_contract_common::BasicAllowance;

        #[test]
        fn requires_providing_valid_grantee_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

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
        use cosmwasm_std::StdError;
        use nym_contracts_common_testing::{AdminExt, RandExt};

        #[test]
        fn requires_providing_valid_grantee_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

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

    #[cfg(test)]
    mod using_allowance {
        use super::*;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, RandExt};
        use nym_pool_contract_common::{BasicAllowance, ExecuteMsg};

        #[test]
        fn requires_at_least_a_single_coin_receiver() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(grantee, ExecuteMsg::UseAllowance { recipients: vec![] });
            assert_eq!(res.unwrap_err(), NymPoolContractError::EmptyUsageRequest);

            Ok(())
        }

        #[test]
        fn requires_valid_coin_for_each_receiver() -> anyhow::Result<()> {
            // 1 bad receiver
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: "invalid-address".to_string(),
                        amount: test.coin(1234),
                    }],
                },
            );
            assert!(res.is_err());

            // 3 receivers, one invalid
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;

            let addr1 = test.generate_account();
            let addr2 = test.generate_account();
            let addr3 = test.generate_account();
            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: addr1.to_string(),
                            amount: test.coin(1234),
                        },
                        TransferRecipient {
                            recipient: addr2.to_string(),
                            amount: test.coin(0),
                        },
                        TransferRecipient {
                            recipient: addr3.to_string(),
                            amount: test.coin(1234),
                        },
                    ],
                },
            );
            assert!(res.is_err());

            // all fine
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: addr1.to_string(),
                            amount: test.coin(1234),
                        },
                        TransferRecipient {
                            recipient: addr2.to_string(),
                            amount: test.coin(1),
                        },
                        TransferRecipient {
                            recipient: addr3.to_string(),
                            amount: test.coin(1234),
                        },
                    ],
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn requires_the_total_to_be_available_for_spending() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let recipient = test.generate_account();

            // contract balance < required
            let grantee = test.add_dummy_grant().grantee;
            test.set_contract_balance(test.coin(100));
            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: recipient.to_string(),
                            amount: test.coin(50),
                        },
                        TransferRecipient {
                            recipient: recipient.to_string(),
                            amount: test.coin(51),
                        },
                    ],
                },
            );
            assert_eq!(
                NymPoolContractError::InsufficientTokens {
                    available: test.coin(100),
                    required: test.coin(101)
                },
                res.unwrap_err()
            );

            // contract balance == required
            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: recipient.to_string(),
                            amount: test.coin(50),
                        },
                        TransferRecipient {
                            recipient: recipient.to_string(),
                            amount: test.coin(50),
                        },
                    ],
                },
            );
            assert!(res.is_ok());

            // contract balance > required
            let grantee = test.add_dummy_grant().grantee;
            test.set_contract_balance(test.coin(100));
            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(50),
                    }],
                },
            );
            assert!(res.is_ok());

            // contract balance > required BUT (contract balance - locked) < required
            let grantee = test.add_dummy_grant().grantee;
            test.set_contract_balance(test.coin(100));
            test.lock_allowance(&grantee, Uint128::new(40));
            let another_grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(
                another_grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(61),
                    }],
                },
            );
            assert_eq!(
                NymPoolContractError::InsufficientTokens {
                    available: test.coin(60),
                    required: test.coin(61)
                },
                res.unwrap_err()
            );

            Ok(())
        }

        #[test]
        fn requires_the_total_to_be_within_spend_limit() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: Some(test.coin(100)),
                expiration_unix_timestamp: None,
            });
            let grantee = test.generate_account();
            let env = test.env();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE.insert_new_grant(
                test.deps_mut(),
                &env,
                &admin,
                &grantee,
                allowance,
            )?;

            let recipient = test.generate_account();
            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(101),
                    }],
                },
            );
            assert_eq!(
                NymPoolContractError::SpendingAboveAllowance,
                res.unwrap_err()
            );

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(100),
                    }],
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn attaches_appropriate_bank_message_for_each_receiver() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let grantee = test.add_dummy_grant().grantee;

            let recipient1 = test.generate_account();
            let recipient2 = test.generate_account();
            let recipient3 = test.generate_account();

            let mut res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: recipient1.to_string(),
                            amount: test.coin(100),
                        },
                        TransferRecipient {
                            recipient: recipient2.to_string(),
                            amount: test.coin(200),
                        },
                        TransferRecipient {
                            recipient: recipient3.to_string(),
                            amount: test.coin(300),
                        },
                    ],
                },
            )?;

            // last
            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, recipient3.to_string());
            assert_eq!(amount, test.coins(300));

            // second
            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, recipient2.to_string());
            assert_eq!(amount, test.coins(200));

            // first
            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, recipient1.to_string());
            assert_eq!(amount, test.coins(100));

            assert!(res.messages.is_empty());

            Ok(())
        }

        #[test]
        fn requires_grant_to_not_be_expired() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let env = test.env();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: Some(env.block.time.seconds() + 1),
            });
            let grantee = test.generate_account();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE.insert_new_grant(
                test.deps_mut(),
                &env,
                &admin,
                &grantee,
                allowance,
            )?;
            test.next_block();

            let recipient = test.generate_account();
            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(100),
                    }],
                },
            );
            assert_eq!(NymPoolContractError::GrantExpired, res.unwrap_err());

            Ok(())
        }
    }

    #[cfg(test)]
    mod withdrawing_from_allowance {
        use super::*;
        use crate::testing::{init_contract_tester, NymPoolContractTesterExt};
        use cosmwasm_std::coin;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, DenomExt, RandExt};
        use nym_pool_contract_common::{BasicAllowance, ExecuteMsg};

        #[test]
        fn requires_valid_coin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: coin(1234, "wtf-denom"),
                },
            );
            assert!(res.is_err());

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(0),
                },
            );
            assert!(res.is_err());

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(123),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn requires_the_amount_to_be_available_for_spending() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            // contract balance < required
            let grantee = test.add_dummy_grant().grantee;
            test.set_contract_balance(test.coin(100));
            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(101),
                },
            );
            assert_eq!(
                NymPoolContractError::InsufficientTokens {
                    available: test.coin(100),
                    required: test.coin(101)
                },
                res.unwrap_err()
            );

            // contract balance == required
            let res = test.execute_raw(
                grantee,
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(100),
                },
            );
            assert!(res.is_ok());

            // contract balance > required
            let grantee = test.add_dummy_grant().grantee;
            test.set_contract_balance(test.coin(100));
            let res = test.execute_raw(
                grantee,
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(50),
                },
            );
            assert!(res.is_ok());

            // contract balance > required BUT (contract balance - locked) < required
            let grantee = test.add_dummy_grant().grantee;
            test.set_contract_balance(test.coin(100));
            test.lock_allowance(&grantee, Uint128::new(40));
            let another_grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(
                another_grantee,
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(61),
                },
            );
            assert_eq!(
                NymPoolContractError::InsufficientTokens {
                    available: test.coin(60),
                    required: test.coin(61)
                },
                res.unwrap_err()
            );

            Ok(())
        }

        #[test]
        fn requires_the_amount_to_be_within_spend_limit() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: Some(test.coin(100)),
                expiration_unix_timestamp: None,
            });
            let grantee = test.generate_account();
            let env = test.env();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE.insert_new_grant(
                test.deps_mut(),
                &env,
                &admin,
                &grantee,
                allowance,
            )?;

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(101),
                },
            );
            assert_eq!(
                NymPoolContractError::SpendingAboveAllowance,
                res.unwrap_err()
            );

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(100),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn attaches_appropriate_bank_message() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let grantee = test.add_dummy_grant().grantee;

            let mut res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(100),
                },
            )?;

            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, grantee.to_string());
            assert_eq!(amount, test.coins(100));

            assert!(res.messages.is_empty());

            Ok(())
        }

        #[test]
        fn requires_grant_to_not_be_expired() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let env = test.env();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: Some(env.block.time.seconds() + 1),
            });
            let grantee = test.generate_account();
            let env = test.env();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE.insert_new_grant(
                test.deps_mut(),
                &env,
                &admin,
                &grantee,
                allowance,
            )?;
            test.next_block();

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawAllowance {
                    amount: test.coin(101),
                },
            );
            assert_eq!(NymPoolContractError::GrantExpired, res.unwrap_err());

            Ok(())
        }
    }

    #[test]
    fn locking_allowance() -> anyhow::Result<()> {
        // internals got tested in storage tests, so this is mostly about checking events (TODO)
        let mut test = init_contract_tester();
        let grantee = test.add_dummy_grant().grantee;

        let res = test.execute_raw(
            grantee.clone(),
            ExecuteMsg::LockAllowance {
                amount: test.coin(100),
            },
        );
        assert!(res.is_ok());

        assert_eq!(
            NYM_POOL_STORAGE
                .locked
                .grantee_locked(test.storage(), &grantee)?,
            Uint128::new(100)
        );

        Ok(())
    }

    #[test]
    fn unlocking_allowance() -> anyhow::Result<()> {
        // internals got tested in storage tests, so this is mostly about checking events (TODO)
        let mut test = init_contract_tester();
        let grantee = test.add_dummy_grant().grantee;
        test.lock_allowance(&grantee, Uint128::new(100));

        let res = test.execute_raw(
            grantee.clone(),
            ExecuteMsg::UnlockAllowance {
                amount: test.coin(40),
            },
        );
        assert!(res.is_ok());

        assert_eq!(
            NYM_POOL_STORAGE
                .locked
                .grantee_locked(test.storage(), &grantee)?,
            Uint128::new(60)
        );

        Ok(())
    }

    #[cfg(test)]
    mod using_locked_allowance {
        use super::*;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, RandExt};
        use nym_pool_contract_common::BasicAllowance;

        #[test]
        fn requires_at_least_a_single_coin_receiver() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseLockedAllowance { recipients: vec![] },
            );
            assert_eq!(res.unwrap_err(), NymPoolContractError::EmptyUsageRequest);

            Ok(())
        }

        #[test]
        fn requires_valid_coin_for_each_receiver() -> anyhow::Result<()> {
            // 1 bad receiver
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(10000));

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: "invalid-address".to_string(),
                        amount: test.coin(1234),
                    }],
                },
            );
            assert!(res.is_err());

            // 3 receivers, one invalid
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(10000));

            let addr1 = test.generate_account();
            let addr2 = test.generate_account();
            let addr3 = test.generate_account();
            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: addr1.to_string(),
                            amount: test.coin(1234),
                        },
                        TransferRecipient {
                            recipient: addr2.to_string(),
                            amount: test.coin(0),
                        },
                        TransferRecipient {
                            recipient: addr3.to_string(),
                            amount: test.coin(1234),
                        },
                    ],
                },
            );
            assert!(res.is_err());

            // all fine
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(10000));

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: addr1.to_string(),
                            amount: test.coin(1234),
                        },
                        TransferRecipient {
                            recipient: addr2.to_string(),
                            amount: test.coin(1),
                        },
                        TransferRecipient {
                            recipient: addr3.to_string(),
                            amount: test.coin(1234),
                        },
                    ],
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn requires_the_total_to_be_locked_by_grantee() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(100));

            let recipient = test.generate_account();
            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(101),
                    }],
                },
            );
            assert_eq!(
                NymPoolContractError::InsufficientLockedTokens {
                    grantee: grantee.to_string(),
                    locked: Uint128::new(100),
                    requested: Uint128::new(101),
                },
                res.unwrap_err()
            );

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(100),
                    }],
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn attaches_appropriate_bank_message_for_each_receiver() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(10000));

            let recipient1 = test.generate_account();
            let recipient2 = test.generate_account();
            let recipient3 = test.generate_account();

            let mut res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![
                        TransferRecipient {
                            recipient: recipient1.to_string(),
                            amount: test.coin(100),
                        },
                        TransferRecipient {
                            recipient: recipient2.to_string(),
                            amount: test.coin(200),
                        },
                        TransferRecipient {
                            recipient: recipient3.to_string(),
                            amount: test.coin(300),
                        },
                    ],
                },
            )?;

            // last
            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, recipient3.to_string());
            assert_eq!(amount, test.coins(300));

            // second
            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, recipient2.to_string());
            assert_eq!(amount, test.coins(200));

            // first
            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, recipient1.to_string());
            assert_eq!(amount, test.coins(100));

            assert!(res.messages.is_empty());

            Ok(())
        }

        #[test]
        fn requires_grant_to_not_be_expired() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let env = test.env();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: Some(env.block.time.seconds() + 1),
            });
            let grantee = test.generate_account();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE.insert_new_grant(
                test.deps_mut(),
                &env,
                &admin,
                &grantee,
                allowance,
            )?;
            test.lock_allowance(&grantee, Uint128::new(10000));
            test.next_block();

            let recipient = test.generate_account();
            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::UseLockedAllowance {
                    recipients: vec![TransferRecipient {
                        recipient: recipient.to_string(),
                        amount: test.coin(100),
                    }],
                },
            );
            assert_eq!(NymPoolContractError::GrantExpired, res.unwrap_err());

            Ok(())
        }
    }

    #[cfg(test)]
    mod withdrawing_from_locked_allowance {
        use super::*;
        use cosmwasm_std::coin;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, RandExt};
        use nym_pool_contract_common::BasicAllowance;

        #[test]
        fn requires_valid_coin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(10000));

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: coin(1234, "wtf-denom"),
                },
            );
            assert!(res.is_err());

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: test.coin(0),
                },
            );
            assert!(res.is_err());

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: test.coin(123),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn attaches_appropriate_bank_message() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(10000));

            let mut res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: test.coin(100),
                },
            )?;

            let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
                res.messages.pop().unwrap().msg
            else {
                panic!("invalid message")
            };
            assert_eq!(to_address, grantee.to_string());
            assert_eq!(amount, test.coins(100));

            assert!(res.messages.is_empty());

            Ok(())
        }

        #[test]
        fn requires_grant_to_not_be_expired() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let env = test.env();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: Some(env.block.time.seconds() + 1),
            });
            let grantee = test.generate_account();
            let env = test.env();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE.insert_new_grant(
                test.deps_mut(),
                &env,
                &admin,
                &grantee,
                allowance,
            )?;
            test.lock_allowance(&grantee, Uint128::new(10000));

            test.next_block();

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: test.coin(101),
                },
            );
            assert_eq!(NymPoolContractError::GrantExpired, res.unwrap_err());

            Ok(())
        }

        #[test]
        fn requires_the_amount_to_be_locked_by_grantee() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.add_dummy_grant().grantee;
            test.lock_allowance(&grantee, Uint128::new(100));

            let res = test.execute_raw(
                grantee.clone(),
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: test.coin(101),
                },
            );
            assert_eq!(
                NymPoolContractError::InsufficientLockedTokens {
                    grantee: grantee.to_string(),
                    locked: Uint128::new(100),
                    requested: Uint128::new(101),
                },
                res.unwrap_err()
            );

            let res = test.execute_raw(
                grantee,
                ExecuteMsg::WithdrawLockedAllowance {
                    amount: test.coin(100),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }
    }

    #[test]
    fn adding_new_granter() -> anyhow::Result<()> {
        let mut test = init_contract_tester();
        let bad_address = "foomp";
        let good_address = test.generate_account();

        // requires valid address
        let res = test.execute_raw(
            test.admin_unchecked(),
            ExecuteMsg::AddNewGranter {
                granter: bad_address.to_string(),
            },
        );
        assert!(res.is_err());

        let res = test.execute_raw(
            test.admin_unchecked(),
            ExecuteMsg::AddNewGranter {
                granter: good_address.to_string(),
            },
        );
        assert!(res.is_ok());

        // introduces new granter
        assert!(NYM_POOL_STORAGE
            .granters
            .may_load(test.storage(), good_address)?
            .is_some());

        Ok(())
    }

    #[test]
    fn revoking_granter() -> anyhow::Result<()> {
        let mut test = init_contract_tester();
        let bad_address = "foomp";
        let good_address = test.generate_account();
        let granter_address = test.generate_account();
        test.add_granter(&granter_address);

        // requires valid address
        let res = test.execute_raw(
            test.admin_unchecked(),
            ExecuteMsg::RevokeGranter {
                granter: bad_address.to_string(),
            },
        );
        assert!(res.is_err());

        // requires an actual granter
        let res = test.execute_raw(
            test.admin_unchecked(),
            ExecuteMsg::RevokeGranter {
                granter: good_address.to_string(),
            },
        );
        assert!(res.is_err());

        // revokes the granter
        let res = test.execute_raw(
            test.admin_unchecked(),
            ExecuteMsg::RevokeGranter {
                granter: granter_address.to_string(),
            },
        );
        assert!(res.is_ok());

        assert!(NYM_POOL_STORAGE
            .granters
            .may_load(test.storage(), granter_address)?
            .is_none());

        Ok(())
    }

    #[cfg(test)]
    mod removing_expired {
        use super::*;
        use crate::testing::{init_contract_tester, NymPoolContract, NymPoolContractTesterExt};
        use nym_contracts_common_testing::{ChainOpts, ContractOpts, ContractTester, RandExt};
        use nym_pool_contract_common::{BasicAllowance, GranteeAddress};

        fn setup_with_expired_grant() -> (ContractTester<NymPoolContract>, GranteeAddress) {
            let mut test = init_contract_tester();
            let env = test.env();
            let allowance = Allowance::Basic(BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: Some(env.block.time.seconds() + 1),
            });
            let grantee = test.generate_account();
            let admin = test.admin_unchecked();
            NYM_POOL_STORAGE
                .insert_new_grant(test.deps_mut(), &env, &admin, &grantee, allowance)
                .unwrap();
            test.next_block();
            (test, grantee)
        }

        #[test]
        fn requires_valid_grantee_address() -> anyhow::Result<()> {
            let (mut test, grantee) = setup_with_expired_grant();
            let sender = test.generate_account();
            let res = test.execute_raw(
                sender.clone(),
                ExecuteMsg::RemoveExpiredGrant {
                    grantee: "bad grantee".to_string(),
                },
            );
            assert!(res.is_err());

            let res = test.execute_raw(
                sender,
                ExecuteMsg::RemoveExpiredGrant {
                    grantee: grantee.to_string(),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn requires_grant_to_actually_exist_and_be_expired() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let sender = test.generate_account();
            let grantee = test.add_dummy_grant().grantee;
            let not_grantee = test.generate_account();

            // doesn't exist
            let res = test.execute_raw(
                sender.clone(),
                ExecuteMsg::RemoveExpiredGrant {
                    grantee: not_grantee.to_string(),
                },
            );
            assert!(res.is_err());

            // exists but not expired
            let res = test.execute_raw(
                sender.clone(),
                ExecuteMsg::RemoveExpiredGrant {
                    grantee: grantee.to_string(),
                },
            );
            assert!(res.is_err());

            Ok(())
        }

        #[test]
        fn removes_the_grant() -> anyhow::Result<()> {
            let (mut test, grantee) = setup_with_expired_grant();
            let sender = test.generate_account();

            assert!(NYM_POOL_STORAGE
                .grants
                .may_load(test.storage(), grantee.clone())?
                .is_some());

            test.execute_raw(
                sender.clone(),
                ExecuteMsg::RemoveExpiredGrant {
                    grantee: grantee.to_string(),
                },
            )?;

            assert!(NYM_POOL_STORAGE
                .grants
                .may_load(test.storage(), grantee)?
                .is_none());

            Ok(())
        }
    }
}
