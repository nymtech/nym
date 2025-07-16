// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE;
use cosmwasm_std::{DepsMut, Env, Event, MessageInfo, Response};
use nym_offline_signers_common::NymOfflineSignersContractError;

pub fn try_update_contract_admin(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, NymOfflineSignersContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?;

    Ok(res)
}

pub fn try_propose_or_vote(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    signer: String,
) -> Result<Response, NymOfflineSignersContractError> {
    let signer = deps.api.addr_validate(&signer)?;

    let reached_quorum =
        NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.propose_or_vote(deps, env, info.sender, signer)?;

    Ok(Response::new().add_event(
        Event::new("offline_signer_vote")
            .add_attribute("quorum_reached", reached_quorum.to_string()),
    ))
}

pub fn try_reset_offline_status(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, NymOfflineSignersContractError> {
    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.reset_offline_status(deps, env, info.sender)?;

    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod updating_contract_admin {
        use super::*;
        use crate::testing::init_contract_tester;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
        use nym_offline_signers_common::ExecuteMsg;

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
                    },
                )
                .unwrap_err();

            assert_eq!(
                res,
                NymOfflineSignersContractError::Admin(AdminError::NotAdmin {})
            );

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
            let mut test = init_contract_tester();

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
}
