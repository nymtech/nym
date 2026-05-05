// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! State-mutating execute handlers. Each entry is currently a stub returning
//! an empty response; concrete implementations will be filled in as the
//! corresponding tickets land.

use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use node_families_contract_common::{Config, NodeFamiliesContractError, NodeFamilyId};
use nym_mixnet_contract_common::NodeId;

/// Replace the contract's runtime [`Config`]. Restricted to the contract admin.
pub(crate) fn try_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    config: Config,
) -> Result<Response, NodeFamiliesContractError> {
    let storage = NodeFamiliesStorage::new();
    storage
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;
    storage.config.save(deps.storage, &config)?;
    Ok(Response::default())
}

pub(crate) fn try_create_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    description: String,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, name, description);
    Ok(Response::default())
}

pub(crate) fn try_disband_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info);
    Ok(Response::default())
}

pub(crate) fn try_invite_to_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_revoke_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_accept_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, family_id, node_id);
    Ok(Response::default())
}

pub(crate) fn try_reject_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, family_id, node_id);
    Ok(Response::default())
}

pub(crate) fn try_leave_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_kick_from_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::init_contract_tester;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::message_info;
    use cw_controllers::AdminError;
    use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};

    fn updated_config() -> Config {
        Config {
            create_family_fee: coin(999, "unym"),
            family_name_length_limit: 1,
            family_description_length_limit: 2,
        }
    }

    #[test]
    fn admin_can_replace_the_config() {
        let mut tester = init_contract_tester();
        let admin = tester.admin_msg();
        let new_config = updated_config();
        let env = tester.env();
        let res = try_update_config(tester.deps_mut(), env, admin, new_config.clone());
        assert!(res.is_ok());

        let stored = NodeFamiliesStorage::new()
            .config
            .load(tester.deps().storage)
            .unwrap();
        assert_eq!(stored, new_config);
    }

    #[test]
    fn non_admin_cannot_update_the_config() {
        let mut tester = init_contract_tester();
        let not_admin = tester.generate_account();
        let not_admin = message_info(&not_admin, &[]);

        let original = NodeFamiliesStorage::new()
            .config
            .load(tester.deps().storage)
            .unwrap();

        let env = tester.env();
        let err =
            try_update_config(tester.deps_mut(), env, not_admin, updated_config()).unwrap_err();

        assert_eq!(
            err,
            NodeFamiliesContractError::Admin(AdminError::NotAdmin {})
        );

        // config left untouched
        let stored = NodeFamiliesStorage::new()
            .config
            .load(tester.deps().storage)
            .unwrap();
        assert_eq!(stored, original);
    }
}
