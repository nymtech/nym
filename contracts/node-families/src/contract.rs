// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! CosmWasm entry points for the node families contract.

use crate::queries::{
    query_all_past_invitations_paged, query_all_pending_invitations_paged, query_families_paged,
    query_family_by_id, query_family_by_name, query_family_by_owner, query_family_members_paged,
    query_family_membership, query_past_invitations_for_family_paged,
    query_past_invitations_for_node_paged, query_past_members_for_family_paged,
    query_past_members_for_node_paged, query_pending_invitation,
    query_pending_invitations_for_family_paged, query_pending_invitations_for_node_paged,
};
use crate::storage::NodeFamiliesStorage;
use crate::transactions::{
    try_accept_family_invitation, try_create_family, try_disband_family, try_handle_node_unbonding,
    try_invite_to_family, try_kick_from_family, try_leave_family, try_reject_family_invitation,
    try_revoke_family_invitation, try_update_config,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use node_families_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NodeFamiliesContractError, QueryMsg,
};
use nym_contracts_common::set_build_information;

const CONTRACT_NAME: &str = "crate:nym-node-families-contract";

/// Contract semver, taken from `Cargo.toml` at build time. Bumped on every
/// release; recorded in cw2 storage so migrations can detect the source version.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One-time initialisation of contract storage on code instantiation.
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NodeFamiliesContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let mixnet_contract_address = deps.api.addr_validate(&msg.mixnet_contract_address)?;

    NodeFamiliesStorage::new().initialise(
        deps,
        info.sender,
        mixnet_contract_address,
        msg.config,
    )?;

    Ok(Response::default())
}

/// State-mutating dispatcher. Concrete handlers live in [`crate::transactions`]
/// and are wired up here as variants are added to [`ExecuteMsg`].
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NodeFamiliesContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => try_update_config(deps, env, info, config),
        ExecuteMsg::CreateFamily { name, description } => {
            try_create_family(deps, env, info, name, description)
        }
        ExecuteMsg::DisbandFamily {} => try_disband_family(deps, env, info),
        ExecuteMsg::InviteToFamily {
            node_id,
            validity_secs,
        } => try_invite_to_family(deps, env, info, node_id, validity_secs),
        ExecuteMsg::RevokeFamilyInvitation { node_id } => {
            try_revoke_family_invitation(deps, env, info, node_id)
        }
        ExecuteMsg::AcceptFamilyInvitation { family_id, node_id } => {
            try_accept_family_invitation(deps, env, info, family_id, node_id)
        }
        ExecuteMsg::RejectFamilyInvitation { family_id, node_id } => {
            try_reject_family_invitation(deps, env, info, family_id, node_id)
        }
        ExecuteMsg::LeaveFamily { node_id } => try_leave_family(deps, env, info, node_id),
        ExecuteMsg::KickFromFamily { node_id } => try_kick_from_family(deps, env, info, node_id),
        ExecuteMsg::OnNymNodeUnbond { node_id } => {
            try_handle_node_unbonding(deps, env, info, node_id)
        }
    }
}

/// Read-only dispatcher. Concrete handlers live in [`crate::queries`] and are
/// wired up here as variants are added to [`QueryMsg`].
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, NodeFamiliesContractError> {
    match msg {
        QueryMsg::GetFamilyById { family_id } => {
            Ok(to_json_binary(&query_family_by_id(deps, family_id)?)?)
        }
        QueryMsg::GetFamilyByOwner { owner } => {
            Ok(to_json_binary(&query_family_by_owner(deps, owner)?)?)
        }
        QueryMsg::GetFamilyByName { name } => {
            Ok(to_json_binary(&query_family_by_name(deps, name)?)?)
        }
        QueryMsg::GetFamilyMembership { node_id } => {
            Ok(to_json_binary(&query_family_membership(deps, node_id)?)?)
        }
        QueryMsg::GetFamilyMembersPaged {
            family_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_family_members_paged(
            deps,
            family_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetPendingInvitation { family_id, node_id } => Ok(to_json_binary(
            &query_pending_invitation(deps, env, family_id, node_id)?,
        )?),
        QueryMsg::GetPendingInvitationsForFamilyPaged {
            family_id,
            start_after,
            limit,
        } => Ok(to_json_binary(
            &query_pending_invitations_for_family_paged(deps, env, family_id, start_after, limit)?,
        )?),
        QueryMsg::GetPendingInvitationsForNodePaged {
            node_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_pending_invitations_for_node_paged(
            deps,
            env,
            node_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetAllPendingInvitationsPaged { start_after, limit } => Ok(to_json_binary(
            &query_all_pending_invitations_paged(deps, env, start_after, limit)?,
        )?),
        QueryMsg::GetPastInvitationsForFamilyPaged {
            family_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_past_invitations_for_family_paged(
            deps,
            family_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetPastInvitationsForNodePaged {
            node_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_past_invitations_for_node_paged(
            deps,
            node_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetAllPastInvitationsPaged { start_after, limit } => Ok(to_json_binary(
            &query_all_past_invitations_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::GetPastMembersForFamilyPaged {
            family_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_past_members_for_family_paged(
            deps,
            family_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetPastMembersForNodePaged {
            node_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_past_members_for_node_paged(
            deps,
            node_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetFamiliesPaged { start_after, limit } => Ok(to_json_binary(
            &query_families_paged(deps, start_after, limit)?,
        )?),
    }
}

/// Migration entry point.
///
/// Refreshes recorded build information and ensures the existing on-chain
/// contract version is at most the current `CONTRACT_VERSION` (i.e. forbids
/// downgrades). Any data migrations are dispatched via
/// [`crate::queued_migrations`].
#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NodeFamiliesContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contract_instantiation {
        use super::*;
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
        use node_families_contract_common::Config;

        fn mock_config() -> Config {
            Config {
                create_family_fee: coin(123, "unym"),
                family_name_length_limit: 20,
                family_description_length_limit: 100,
                default_invitation_validity_secs: 24 * 60 * 60,
            }
        }

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let some_sender = deps.api.addr_make("some_sender");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                InstantiateMsg {
                    config: mock_config(),
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                },
            )?;

            let deps = deps.as_ref();

            NodeFamiliesStorage::new()
                .contract_admin
                .assert_admin(deps, &some_sender)?;

            Ok(())
        }

        #[test]
        fn persists_the_provided_config() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let sender = deps.api.addr_make("some_sender");
            let config = mock_config();

            instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    config: config.clone(),
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                },
            )?;

            let stored = NodeFamiliesStorage::new()
                .config
                .load(deps.as_ref().storage)?;
            assert_eq!(stored, config);

            Ok(())
        }

        #[test]
        fn persists_the_validated_mixnet_contract_address() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let sender = deps.api.addr_make("some_sender");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    config: mock_config(),
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                },
            )?;

            let stored = NodeFamiliesStorage::new()
                .mixnet_contract_address
                .load(deps.as_ref().storage)?;
            assert_eq!(stored, mixnet_contract_address);

            Ok(())
        }

        #[test]
        fn errors_on_invalid_mixnet_contract_address() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let sender = deps.api.addr_make("some_sender");

            let res = instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    config: mock_config(),
                    mixnet_contract_address: "not-a-valid-bech32-address".to_string(),
                },
            );

            assert!(res.is_err());
        }

        #[test]
        fn records_the_cw2_contract_version() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let sender = deps.api.addr_make("some_sender");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    config: mock_config(),
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                },
            )?;

            let version = cw2::get_contract_version(deps.as_ref().storage)?;
            assert_eq!(version.contract, CONTRACT_NAME);
            assert_eq!(version.version, CONTRACT_VERSION);

            Ok(())
        }
    }
}
