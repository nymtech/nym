// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! CosmWasm entry points for the nym directory contract.

use crate::queries::{
    query_admin, query_all_entries, query_allowed_labels, query_curated_entries_paged,
    query_curated_entry, query_digest, query_node_entries, query_node_entries_paged,
    query_node_entry, query_sequence, query_snapshot_interval,
};
use crate::storage::NYM_DIRECTORY_CONTRACT_STORAGE;
use crate::transactions::{
    try_delete_node_entry, try_handle_node_unbonding, try_remove_curated_entry, try_remove_label,
    try_set_curated_entry, try_set_label, try_set_node_entry, try_update_admin,
    try_update_snapshot_interval,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_directory_contract_common::constants::DEFAULT_SNAPSHOT_INTERVAL;
use nym_directory_contract_common::{
    DirectoryContractError, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-directory-contract";

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
) -> Result<Response, DirectoryContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let mixnet_contract_address = deps.api.addr_validate(&msg.mixnet_contract_address)?;

    NYM_DIRECTORY_CONTRACT_STORAGE.initialise(
        deps,
        info.sender,
        mixnet_contract_address,
        msg.snapshot_interval.unwrap_or(DEFAULT_SNAPSHOT_INTERVAL),
        msg.initial_labels,
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
) -> Result<Response, DirectoryContractError> {
    match msg {
        ExecuteMsg::SetNodeEntry {
            node_id,
            label,
            data,
            sequence,
            signature,
        } => try_set_node_entry(deps, env, node_id, label, data, sequence, signature),
        ExecuteMsg::DeleteNodeEntry {
            node_id,
            label,
            sequence,
            signature,
        } => try_delete_node_entry(deps, node_id, label, sequence, signature),
        ExecuteMsg::SetCuratedEntry { key, data } => try_set_curated_entry(deps, info, key, data),
        ExecuteMsg::RemoveCuratedEntry { key } => try_remove_curated_entry(deps, info, key),
        ExecuteMsg::SetLabel { label, max_size } => try_set_label(deps, info, label, max_size),
        ExecuteMsg::RemoveLabel { label } => try_remove_label(deps, info, label),
        ExecuteMsg::UpdateAdmin { admin } => try_update_admin(deps, info, admin),
        ExecuteMsg::OnNymNodeUnbond { node_id } => try_handle_node_unbonding(deps, info, node_id),
        ExecuteMsg::UpdateSnapshotInterval { interval } => {
            try_update_snapshot_interval(deps, info, interval)
        }
    }
}

/// Read-only dispatcher. Concrete handlers live in [`crate::queries`] and are
/// wired up here as variants are added to [`QueryMsg`].
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, DirectoryContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::NodeEntry { node_id, label } => {
            Ok(to_json_binary(&query_node_entry(deps, node_id, label)?)?)
        }
        QueryMsg::CuratedEntry { key } => Ok(to_json_binary(&query_curated_entry(deps, key)?)?),
        QueryMsg::NodeEntries { node_id } => {
            Ok(to_json_binary(&query_node_entries(deps, node_id)?)?)
        }
        QueryMsg::NodeEntriesPaged { start_after, limit } => Ok(to_json_binary(
            &query_node_entries_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::CuratedEntriesPaged { start_after, limit } => Ok(to_json_binary(
            &query_curated_entries_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::AllEntries { start_after, limit } => Ok(to_json_binary(&query_all_entries(
            deps,
            start_after,
            limit,
        )?)?),
        QueryMsg::Sequence { node_id } => Ok(to_json_binary(&query_sequence(deps, node_id)?)?),
        QueryMsg::Digest {} => Ok(to_json_binary(&query_digest(deps)?)?),
        QueryMsg::AllowedLabels {} => Ok(to_json_binary(&query_allowed_labels(deps)?)?),
        QueryMsg::SnapshotInterval {} => Ok(to_json_binary(&query_snapshot_interval(deps)?)?),
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
) -> Result<Response, DirectoryContractError> {
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
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

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
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                    snapshot_interval: None,
                    initial_labels: vec![],
                },
            )?;

            let deps = deps.as_ref();

            NYM_DIRECTORY_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps, &some_sender)?;

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
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                    snapshot_interval: None,
                    initial_labels: vec![],
                },
            )?;

            let stored = NYM_DIRECTORY_CONTRACT_STORAGE
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
                    mixnet_contract_address: "not-a-valid-bech32-address".to_string(),
                    snapshot_interval: None,
                    initial_labels: vec![],
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
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                    snapshot_interval: None,
                    initial_labels: vec![],
                },
            )?;

            let version = cw2::get_contract_version(deps.as_ref().storage)?;
            assert_eq!(version.contract, CONTRACT_NAME);
            assert_eq!(version.version, CONTRACT_VERSION);

            Ok(())
        }

        #[test]
        fn defaults_the_snapshot_interval_when_unset() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let sender = deps.api.addr_make("some_sender");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                    snapshot_interval: None,
                    initial_labels: vec![],
                },
            )?;

            assert_eq!(
                query_snapshot_interval(deps.as_ref())?.interval,
                DEFAULT_SNAPSHOT_INTERVAL
            );
            Ok(())
        }

        #[test]
        fn persists_an_explicit_snapshot_interval() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let sender = deps.api.addr_make("some_sender");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                    snapshot_interval: Some(250),
                    initial_labels: vec![],
                },
            )?;

            assert_eq!(query_snapshot_interval(deps.as_ref())?.interval, 250);
            Ok(())
        }

        #[test]
        fn rejects_a_zero_snapshot_interval() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mixnet_contract_address = deps.api.addr_make("mixnet-contract");
            let sender = deps.api.addr_make("some_sender");

            let res = instantiate(
                deps.as_mut(),
                env,
                message_info(&sender, &[]),
                InstantiateMsg {
                    mixnet_contract_address: mixnet_contract_address.to_string(),
                    snapshot_interval: Some(0),
                    initial_labels: vec![],
                },
            );

            assert_eq!(
                res.unwrap_err(),
                DirectoryContractError::InvalidSnapshotInterval
            );
        }
    }
}
