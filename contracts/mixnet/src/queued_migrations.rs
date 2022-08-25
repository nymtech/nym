// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::queries::query_mixnode_delegation;
use crate::delegations::storage::delegations;
use crate::error::ContractError;
use crate::mixnodes::storage::mixnodes;
use cosmwasm_std::{wasm_execute, Addr, BankMsg, DepsMut, Env, Response, SubMsg};
use cw_storage_plus::Map;
use mixnet_contract_common::delegation::generate_storage_key;
use mixnet_contract_common::{IdentityKey, MigrateMsg, SpecialV2ExecuteMsg, V2MigrationOperation};
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

const MIGRATED_MIXNODES: Map<IdentityKey, u8> = Map::new("migrated-mixnodes");
type OwnerXorProxy = Vec<u8>;
const MIGRATED_DELEGATES: Map<(IdentityKey, OwnerXorProxy), u8> = Map::new("migrated-delegates");

fn migrate_operator(
    deps: &mut DepsMut,
    v2_mixnet_contract: &str,
    node_identity: String,
    response: &mut Response,
) -> Result<(), ContractError> {
    if MIGRATED_MIXNODES.has(deps.storage, node_identity.clone()) {
        // again, panic here because this should never occur and it's incredible dangerous to let it happen
        panic!("mixnode {} has already been migrated!", node_identity);
    }
    MIGRATED_MIXNODES.save(deps.storage, node_identity.clone(), &1u8)?;

    let bond = mixnodes()
        .load(deps.storage, &node_identity)
        .expect("failed to read mixnode bond");

    let pledge = bond.pledge_amount.clone();
    let v2_message = SpecialV2ExecuteMsg::SaveOperator {
        host: bond.mix_node.host,
        mix_port: bond.mix_node.mix_port,
        verloc_port: bond.mix_node.verloc_port,
        http_api_port: bond.mix_node.http_api_port,
        sphinx_key: bond.mix_node.sphinx_key,
        identity_key: bond.mix_node.identity_key,
        version: bond.mix_node.version,
        pledge_amount: bond.pledge_amount,
        owner: bond.owner,
        block_height: bond.block_height,
        proxy: bond.proxy,
    };

    // TODO: do we need separate BankMsg here, or can we just use 'funds' here directly?
    let wasm_msg = wasm_execute(v2_mixnet_contract, &v2_message, vec![pledge])
        .expect("failed to serialize mixnode migration msg");
    response.messages.push(SubMsg::new(wasm_msg));
    Ok(())
}

fn is_proxy_vesting(proxy: Option<&Addr>, vesting_contract: &str) -> bool {
    if let Some(proxy) = proxy {
        if proxy.as_ref() == vesting_contract {
            return true;
        }
    }
    false
}

fn migrate_delegator(
    deps: &mut DepsMut,
    v2_mixnet_contract: &str,
    vesting_contract: &str,
    address: Addr,
    node_identity: String,
    proxy: Option<Addr>,
    new_mix_id: Option<u64>,
    response: &mut Response,
) -> Result<(), ContractError> {
    let owner_proxy = generate_storage_key(&address, proxy.as_ref());

    let storage_key = (node_identity.clone(), owner_proxy);
    if MIGRATED_DELEGATES.has(deps.storage, storage_key.clone()) {
        // again, panic here because this should never occur and it's incredible dangerous to let it happen
        panic!(
            "delegator {}/{} has already been migrated!",
            address, node_identity
        );
    }
    MIGRATED_DELEGATES.save(deps.storage, storage_key, &1u8)?;

    // there should only be one (as we ensured it during previous migration steps, if not, then somebody is not following the migration instructions
    // and we're in an inconsistent state)
    // also this entry MUST exist as we're explicitly migrating this one
    let mut delegation = query_mixnode_delegation(
        deps.storage,
        deps.api,
        node_identity.clone(),
        address.clone().into_string(),
        proxy.map(Addr::into_string),
    )
    .expect("specified delegation doesn't exist!!");

    if delegation.len() != 1 {
        panic!("the universal compound hasn't been run prior to this migration!!")
    }

    // take ownership of the one and only entry
    let delegation = delegation.pop().unwrap();

    // if mix_id is `None`, it means target mixnode doesn't exist anymore -> return the tokens
    // otherwise attempt to migrate it into the new contract
    if let Some(migrated_mix_id) = new_mix_id {
        if is_proxy_vesting(delegation.proxy.as_ref(), vesting_contract) {
            let vesting_update = VestingContractExecuteMsg::AuthorisedUpdateToV2 {
                owner: address.into_string(),
                node_identity,
                mix_id: migrated_mix_id,
            };

            let wasm_msg = wasm_execute(vesting_contract, &vesting_update, vec![])
                .expect("failed to serialize vesting migration msg");
            response.messages.push(SubMsg::new(wasm_msg));
        }

        let stake = delegation.amount.clone();
        let v2_message = SpecialV2ExecuteMsg::SaveDelegation {
            owner: delegation.owner,
            mix_id: migrated_mix_id,
            amount: delegation.amount,
            block_height: delegation.block_height,
            proxy: delegation.proxy,
        };

        let wasm_msg = wasm_execute(v2_mixnet_contract, &v2_message, vec![stake])
            .expect("failed to serialize mixnode migration msg");
        response.messages.push(SubMsg::new(wasm_msg));
    } else {
        let mut to_address = delegation.owner.to_string();
        let mut make_bank_msg = true;
        // if the specified proxy matches the vesting contract address -> treat it as "proper" undelegation
        // and send tokens back there
        if let Some(proxy) = delegation.proxy {
            to_address = proxy.to_string();
            if proxy == vesting_contract {
                make_bank_msg = false;
                let vesting_track = VestingContractExecuteMsg::TrackUndelegation {
                    owner: delegation.owner.to_string(),
                    mix_identity: delegation.node_identity,
                    amount: delegation.amount.clone(),
                };
                let wasm_msg = wasm_execute(
                    vesting_contract,
                    &vesting_track,
                    vec![delegation.amount.clone()],
                )?;
                response.messages.push(SubMsg::new(wasm_msg));
            }
        }

        // otherwise just send tokens back to the user / different proxy
        if make_bank_msg {
            let return_tokens = BankMsg::Send {
                to_address,
                amount: vec![delegation.amount],
            };
            response.messages.push(SubMsg::new(return_tokens));
        }
    }
    Ok(())
}

fn remove_operator(
    deps: &mut DepsMut,
    env: &Env,
    node_identity: String,
) -> Result<(), ContractError> {
    if !MIGRATED_MIXNODES.has(deps.storage, node_identity.clone()) {
        // again, panic here because this should never occur and it's incredible dangerous to let it happen
        panic!(
            "attempted to remove mixnode {} without prior migration!",
            node_identity
        );
    }
    mixnodes().remove(deps.storage, &node_identity, env.block.height)?;
    Ok(())
}

fn remove_delegator(
    deps: &mut DepsMut,
    address: Addr,
    node_identity: String,
    proxy: Option<Addr>,
) -> Result<(), ContractError> {
    let owner_proxy = generate_storage_key(&address, proxy.as_ref());

    let storage_key = (node_identity.clone(), owner_proxy);
    if !MIGRATED_DELEGATES.has(deps.storage, storage_key.clone()) {
        // again, panic here because this should never occur and it's incredible dangerous to let it happen
        panic!(
            "attempted to remove delegator {}/{} without prior migration!",
            address, node_identity
        );
    }

    let height = delegations()
        .prefix(storage_key.clone())
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .next()
        .unwrap()
        .unwrap();
    delegations().remove(deps.storage, (storage_key.0, storage_key.1, height))?;
    Ok(())
}

pub fn v2_migration(
    mut deps: DepsMut<'_>,
    env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let mut response = Response::new();

    // note: we're explicitly failing on failures here because we expect EVERYTHING we migrate to exist
    for op in msg.operations {
        match op {
            V2MigrationOperation::MigrateOperator { node_identity } => {
                migrate_operator(
                    &mut deps,
                    &msg.v2_contract_address,
                    node_identity,
                    &mut response,
                )?;
            }
            V2MigrationOperation::MigrateDelegator {
                address,
                node_identity,
                proxy,
                new_mix_id,
            } => migrate_delegator(
                &mut deps,
                &msg.v2_contract_address,
                &msg.vesting_contract_address,
                address,
                node_identity,
                proxy,
                new_mix_id,
                &mut response,
            )?,
            V2MigrationOperation::RemoveOperator { node_identity } => {
                remove_operator(&mut deps, &env, node_identity)?;
            }
            V2MigrationOperation::RemoveDelegator {
                address,
                node_identity,
                proxy,
            } => remove_delegator(&mut deps, address, node_identity, proxy)?,
        }
    }

    Ok(response)
}
