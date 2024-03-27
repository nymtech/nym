// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{coin, Addr, Coin, DepsMut, Env, MessageInfo, Response, Storage};

use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_mixnode_bonding_event, new_mixnode_config_update_event,
    new_mixnode_pending_cost_params_update_event, new_pending_mixnode_unbonding_event,
    new_pending_pledge_decrease_event, new_pending_pledge_increase_event,
};
use mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use mixnet_contract_common::pending_events::{PendingEpochEventKind, PendingIntervalEventKind};
use mixnet_contract_common::{Layer, MixId, MixNode};
use nym_contracts_common::signing::MessageSignature;

use crate::interval::storage as interval_storage;
use crate::interval::storage::push_new_interval_event;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnet_contract_settings::storage::rewarding_denom;
use crate::mixnodes::helpers::{
    get_mixnode_details_by_owner, must_get_mixnode_bond_by_owner, save_new_mixnode,
};
use crate::mixnodes::signature_helpers::verify_mixnode_bonding_signature;
use crate::signing::storage as signing_storage;
use crate::support::helpers::{
    ensure_bonded, ensure_epoch_in_progress_state, ensure_is_authorized, ensure_no_existing_bond,
    ensure_no_pending_pledge_changes, ensure_proxy_match, ensure_sent_by_vesting_contract,
    validate_pledge,
};

use super::storage;

pub(crate) fn update_mixnode_layer(
    mix_id: MixId,
    layer: Layer,
    storage: &mut dyn Storage,
) -> Result<(), MixnetContractError> {
    let bond = if let Some(bond_information) = storage::mixnode_bonds().may_load(storage, mix_id)? {
        bond_information
    } else {
        return Err(MixnetContractError::MixNodeBondNotFound { mix_id });
    };
    let mut updated_bond = bond.clone();
    updated_bond.layer = layer;

    storage::mixnode_bonds().replace(storage, bond.mix_id, Some(&updated_bond), Some(&bond))?;
    Ok(())
}

pub fn assign_mixnode_layer(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: MixId,
    layer: Layer,
) -> Result<Response, MixnetContractError> {
    ensure_is_authorized(&info.sender, deps.storage)?;

    update_mixnode_layer(mix_id, layer, deps.storage)?;

    Ok(Response::default())
}

pub fn try_add_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
    owner_signature: MessageSignature,
) -> Result<Response, MixnetContractError> {
    _try_add_mixnode(
        deps,
        env,
        mix_node,
        cost_params,
        info.funds,
        info.sender,
        owner_signature,
        None,
    )
}

pub fn try_add_mixnode_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
    owner: String,
    owner_signature: MessageSignature,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_add_mixnode(
        deps,
        env,
        mix_node,
        cost_params,
        info.funds,
        owner,
        owner_signature,
        Some(proxy),
    )
}

// I'm not entirely sure how to deal with this warning at the current moment
//
// TODO: perhaps also require the user to explicitly provide what it thinks is the current nonce
// so that we could return a better error message if it doesn't match?
#[allow(clippy::too_many_arguments)]
fn _try_add_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    pledge: Vec<Coin>,
    owner: Addr,
    owner_signature: MessageSignature,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::minimum_mixnode_pledge(deps.storage)?;
    let pledge = validate_pledge(pledge, minimum_pledge)?;

    // if the client has an active bonded mixnode or gateway, don't allow bonding
    // note that this has to be done explicitly as `UniqueIndex` constraint would not protect us
    // against attempting to use different node types (i.e. gateways and mixnodes)
    ensure_no_existing_bond(&owner, deps.storage)?;

    // there's no need to explicitly check whether there already exists mixnode with the same
    // identity or sphinx keys as this is going to be done implicitly when attempting to save
    // the bond information due to `UniqueIndex` constraint defined on those fields.

    // check if this sender actually owns the mixnode by checking the signature
    verify_mixnode_bonding_signature(
        deps.as_ref(),
        owner.clone(),
        proxy.clone(),
        pledge.clone(),
        mixnode.clone(),
        cost_params.clone(),
        owner_signature,
    )?;

    // update the signing nonce associated with this sender so that the future signature would be made on the new value
    signing_storage::increment_signing_nonce(deps.storage, owner.clone())?;

    let node_identity = mixnode.identity_key.clone();
    let (node_id, layer) = save_new_mixnode(
        deps.storage,
        env,
        mixnode,
        cost_params,
        owner.clone(),
        proxy.clone(),
        pledge.clone(),
    )?;

    Ok(Response::new().add_event(new_mixnode_bonding_event(
        &owner,
        &proxy,
        &pledge,
        &node_identity,
        node_id,
        layer,
    )))
}

pub fn try_increase_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_increase_pledge(deps, env, info.funds, info.sender, None)
}

pub fn try_increase_pledge_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_increase_pledge(deps, env, info.funds, owner, Some(proxy))
}

pub fn _try_increase_pledge(
    deps: DepsMut<'_>,
    env: Env,
    increase: Vec<Coin>,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let mix_details = get_mixnode_details_by_owner(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond { owner })?;
    let mut pending_changes = mix_details.pending_changes;
    let mix_id = mix_details.mix_id();

    // increasing pledge is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    ensure_proxy_match(&proxy, &mix_details.bond_information.proxy)?;
    ensure_bonded(&mix_details.bond_information)?;
    ensure_no_pending_pledge_changes(&pending_changes)?;

    let rewarding_denom = rewarding_denom(deps.storage)?;
    let pledge_increase = validate_pledge(increase, coin(1, rewarding_denom))?;

    let cosmos_event = new_pending_pledge_increase_event(mix_id, &pledge_increase);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::PledgeMore {
        mix_id,
        amount: pledge_increase,
    };
    let epoch_event_id = interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;
    pending_changes.pledge_change = Some(epoch_event_id);
    storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub fn try_decrease_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    decrease_by: Coin,
) -> Result<Response, MixnetContractError> {
    _try_decrease_pledge(deps, env, decrease_by, info.sender, None)
}

pub fn try_decrease_pledge_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    decrease_by: Coin,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_decrease_pledge(deps, env, decrease_by, owner, Some(proxy))
}

pub fn _try_decrease_pledge(
    deps: DepsMut<'_>,
    env: Env,
    decrease_by: Coin,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let mix_details = get_mixnode_details_by_owner(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond { owner })?;
    let mut pending_changes = mix_details.pending_changes;
    let mix_id = mix_details.mix_id();

    // decreasing pledge is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    ensure_proxy_match(&proxy, &mix_details.bond_information.proxy)?;
    ensure_bonded(&mix_details.bond_information)?;
    ensure_no_pending_pledge_changes(&pending_changes)?;

    let minimum_pledge = mixnet_params_storage::minimum_mixnode_pledge(deps.storage)?;

    // check that the denomination is correct
    if decrease_by.denom != minimum_pledge.denom {
        return Err(MixnetContractError::WrongDenom {
            received: decrease_by.denom,
            expected: minimum_pledge.denom,
        });
    }

    // also check if the request contains non-zero amount
    // (otherwise it's a no-op and we should we waste gas when resolving events?)
    if decrease_by.amount.is_zero() {
        return Err(MixnetContractError::ZeroCoinAmount);
    }

    // decreasing pledge can't result in the new pledge being lower than the minimum amount
    let new_pledge_amount = mix_details
        .original_pledge()
        .amount
        .saturating_sub(decrease_by.amount);
    if new_pledge_amount < minimum_pledge.amount {
        return Err(MixnetContractError::InvalidPledgeReduction {
            current: mix_details.original_pledge().amount,
            decrease_by: decrease_by.amount,
            minimum: minimum_pledge.amount,
            denom: minimum_pledge.denom,
        });
    }

    let cosmos_event = new_pending_pledge_decrease_event(mix_id, &decrease_by);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::DecreasePledge {
        mix_id,
        decrease_by,
    };
    let epoch_event_id = interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;
    pending_changes.pledge_change = Some(epoch_event_id);
    storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub fn try_remove_mixnode_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_remove_mixnode(deps, env, owner, Some(proxy))
}

pub fn try_remove_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_remove_mixnode(deps, env, info.sender, None)
}

pub(crate) fn _try_remove_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &owner)?;
    let pending_changes = storage::PENDING_MIXNODE_CHANGES
        .may_load(deps.storage, existing_bond.mix_id)?
        .unwrap_or_default();

    // unbonding is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    // see if the proxy matches
    ensure_proxy_match(&proxy, &existing_bond.proxy)?;
    ensure_bonded(&existing_bond)?;

    // if there are any pending requests to change the pledge, wait for them to resolve before allowing the unbonding
    ensure_no_pending_pledge_changes(&pending_changes)?;

    // set `is_unbonding` field
    // clippy beta 1.70.0-beta.1 false positive
    #[allow(clippy::redundant_clone)]
    let mut updated_bond = existing_bond.clone();
    updated_bond.is_unbonding = true;
    storage::mixnode_bonds().replace(
        deps.storage,
        existing_bond.mix_id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::UnbondMixnode {
        mix_id: existing_bond.mix_id,
    };
    interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;

    Ok(
        Response::new().add_event(new_pending_mixnode_unbonding_event(
            &existing_bond.owner,
            &existing_bond.proxy,
            existing_bond.identity(),
            existing_bond.mix_id,
        )),
    )
}

pub(crate) fn try_update_mixnode_config(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_config: MixNodeConfigUpdate,
) -> Result<Response, MixnetContractError> {
    let owner = info.sender;
    _try_update_mixnode_config(deps, new_config, owner, None)
}

pub(crate) fn try_update_mixnode_config_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_config: MixNodeConfigUpdate,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let owner = deps.api.addr_validate(&owner)?;
    let proxy = info.sender;
    _try_update_mixnode_config(deps, new_config, owner, Some(proxy))
}

pub(crate) fn _try_update_mixnode_config(
    deps: DepsMut<'_>,
    new_config: MixNodeConfigUpdate,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    ensure_bonded(&existing_bond)?;
    ensure_proxy_match(&proxy, &existing_bond.proxy)?;

    let cfg_update_event =
        new_mixnode_config_update_event(existing_bond.mix_id, &owner, &proxy, &new_config);

    // clippy beta 1.70.0-beta.1 false positive
    #[allow(clippy::redundant_clone)]
    let mut updated_bond = existing_bond.clone();
    updated_bond.mix_node.host = new_config.host;
    updated_bond.mix_node.mix_port = new_config.mix_port;
    updated_bond.mix_node.verloc_port = new_config.verloc_port;
    updated_bond.mix_node.http_api_port = new_config.http_api_port;
    updated_bond.mix_node.version = new_config.version;

    storage::mixnode_bonds().replace(
        deps.storage,
        existing_bond.mix_id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    Ok(Response::new().add_event(cfg_update_event))
}

pub(crate) fn try_update_mixnode_cost_params(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    new_costs: MixNodeCostParams,
) -> Result<Response, MixnetContractError> {
    let owner = info.sender;
    _try_update_mixnode_cost_params(deps, env, new_costs, owner, None)
}

pub(crate) fn try_update_mixnode_cost_params_on_behalf(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_costs: MixNodeCostParams,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let owner = deps.api.addr_validate(&owner)?;
    let proxy = info.sender;
    _try_update_mixnode_cost_params(deps, env, new_costs, owner, Some(proxy))
}

pub(crate) fn _try_update_mixnode_cost_params(
    deps: DepsMut,
    env: Env,
    new_costs: MixNodeCostParams,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // see if the node still exists
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    // changing cost params is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    ensure_proxy_match(&proxy, &existing_bond.proxy)?;
    ensure_bonded(&existing_bond)?;

    let cosmos_event = new_mixnode_pending_cost_params_update_event(
        existing_bond.mix_id,
        &owner,
        &proxy,
        &new_costs,
    );

    // push the interval event
    let interval_event = PendingIntervalEventKind::ChangeMixCostParams {
        mix_id: existing_bond.mix_id,
        new_costs,
    };
    push_new_interval_event(deps.storage, &env, interval_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

#[cfg(test)]
pub mod tests {
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{Order, StdResult, Uint128};

    use mixnet_contract_common::mixnode::PendingMixNodeChanges;
    use mixnet_contract_common::{
        EpochState, EpochStatus, ExecuteMsg, LayerDistribution, Percent,
    };

    use crate::contract::execute;
    use crate::mixnet_contract_settings::storage::minimum_mixnode_pledge;
    use crate::mixnodes::helpers::get_mixnode_details_by_id;
    use crate::support::tests::fixtures::{good_mixnode_pledge, TEST_COIN_DENOM};
    use crate::support::tests::test_helpers::TestSetup;
    use crate::support::tests::{fixtures, test_helpers};

    use super::*;

    #[test]
    fn mixnode_add() {
        let mut test = TestSetup::new();
        let env = test.env();

        let sender = "alice";
        let minimum_pledge = minimum_mixnode_pledge(test.deps().storage).unwrap();
        let mut insufficient_pledge = minimum_pledge.clone();
        insufficient_pledge.amount -= Uint128::new(1000);

        // if we don't send enough funds
        let info = mock_info(sender, &[insufficient_pledge.clone()]);
        let (mixnode, sig, _) =
            test.mixnode_with_signature(sender, Some(vec![insufficient_pledge.clone()]));
        let cost_params = fixtures::mix_node_cost_params_fixture();

        // we are informed that we didn't send enough funds
        let result = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info,
            mixnode.clone(),
            cost_params.clone(),
            sig.clone(),
        );
        assert_eq!(
            result,
            Err(MixnetContractError::InsufficientPledge {
                received: insufficient_pledge,
                minimum: minimum_pledge.clone(),
            })
        );

        // if the signature provided is invalid, the bonding also fails
        let info = mock_info(sender, &[minimum_pledge]);

        // if there was already a mixnode bonded by particular user
        test.add_dummy_mixnode(sender, None);

        // it fails
        let result = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info,
            mixnode,
            cost_params.clone(),
            sig,
        );
        assert_eq!(Err(MixnetContractError::AlreadyOwnsMixnode), result);

        // the same holds if the user already owns a gateway
        let sender2 = "gateway-owner";

        test.add_dummy_gateway(sender2, None);

        let info = mock_info(sender2, &tests::fixtures::good_mixnode_pledge());
        let (mixnode, sig, _) = test.mixnode_with_signature(sender2, None);

        let result = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            mixnode.clone(),
            cost_params.clone(),
            sig.clone(),
        );
        assert_eq!(Err(MixnetContractError::AlreadyOwnsGateway), result);

        // but after he unbonds it, it's all fine again
        let msg = ExecuteMsg::UnbondGateway {};
        execute(test.deps_mut(), env.clone(), info.clone(), msg).unwrap();

        let result = try_add_mixnode(test.deps_mut(), env, info, mixnode, cost_params, sig);
        assert!(result.is_ok());

        // make sure we got assigned the next id (note: we have already bonded a mixnode before in this test)
        let bond =
            must_get_mixnode_bond_by_owner(test.deps().storage, &Addr::unchecked(sender2)).unwrap();
        assert_eq!(2, bond.mix_id);

        // and make sure we're on layer 2 (because it was the next empty one)
        assert_eq!(Layer::Two, bond.layer);

        // and see if the layer distribution matches our expectation
        let expected = LayerDistribution {
            layer1: 1,
            layer2: 1,
            layer3: 0,
        };
        assert_eq!(expected, storage::LAYERS.load(test.deps().storage).unwrap())
    }

    #[test]
    fn adding_mixnode_with_invalid_signatures() {
        let mut test = TestSetup::new();
        let env = test.env();

        let sender = "alice";
        let pledge = good_mixnode_pledge();
        let info = mock_info(sender, pledge.as_ref());

        let (mixnode, signature, _) = test.mixnode_with_signature(sender, Some(pledge.clone()));
        // the above using cost params fixture
        let cost_params = fixtures::mix_node_cost_params_fixture();

        // using different parameters than what the signature was made on
        let mut modified_mixnode = mixnode.clone();
        modified_mixnode.mix_port += 1;
        let res = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info,
            modified_mixnode,
            cost_params.clone(),
            signature.clone(),
        );
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));

        // even stake amount is protected
        let mut different_pledge = pledge.clone();
        different_pledge[0].amount += Uint128::new(12345);

        let info = mock_info(sender, different_pledge.as_ref());
        let res = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info,
            mixnode.clone(),
            cost_params.clone(),
            signature.clone(),
        );
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));

        let other_sender = mock_info("another-sender", pledge.as_ref());
        let res = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            other_sender,
            mixnode.clone(),
            cost_params.clone(),
            signature.clone(),
        );
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));

        // trying to reuse the same signature for another bonding fails (because nonce doesn't match!)
        let info = mock_info(sender, pledge.as_ref());
        let current_nonce =
            signing_storage::get_signing_nonce(test.deps().storage, Addr::unchecked(sender))
                .unwrap();
        assert_eq!(0, current_nonce);
        let res = try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            mixnode.clone(),
            cost_params.clone(),
            signature.clone(),
        );
        assert!(res.is_ok());
        let updated_nonce =
            signing_storage::get_signing_nonce(test.deps().storage, Addr::unchecked(sender))
                .unwrap();
        assert_eq!(1, updated_nonce);

        test.immediately_unbond_mixnode(1);
        let res = try_add_mixnode(test.deps_mut(), env, info, mixnode, cost_params, signature);
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));
    }

    #[test]
    fn mixnode_add_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";
        let (mixnode, sig, _) = test.mixnode_with_signature(owner, None);
        let cost_params = fixtures::mix_node_cost_params_fixture();

        let res = try_add_mixnode_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &good_mixnode_pledge()),
            mixnode,
            cost_params,
            owner.to_string(),
            sig,
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract,
            }
        )
    }

    #[test]
    fn removing_mixnode_cant_be_performed_if_epoch_transition_is_in_progress() {
        let bad_states = vec![
            EpochState::Rewarding {
                last_rewarded: 0,
                final_node_id: 0,
            },
            EpochState::ReconcilingEvents,
            EpochState::AdvancingEpoch,
        ];

        for bad_state in bad_states {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "alice";
            let info = mock_info(owner, &[]);

            test.add_dummy_mixnode(owner, None);

            let mut status = EpochStatus::new(test.rewarding_validator().sender);
            status.state = bad_state;
            interval_storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

            let res = try_remove_mixnode(test.deps_mut(), env.clone(), info);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochAdvancementInProgress { .. })
            ));
        }
    }

    #[test]
    fn mixnode_remove() {
        let mut test = TestSetup::new();
        let env = test.env();

        let owner = "alice";
        let info = mock_info(owner, &[]);

        // trying to remove your mixnode fails if you never had one in the first place
        let res = try_remove_mixnode(test.deps_mut(), env.clone(), info.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(owner)
            })
        );

        let mix_id = test.add_dummy_mixnode(owner, None);
        let vesting_contract = test.vesting_contract();

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_remove_mixnode_on_behalf(
            test.deps_mut(),
            env.clone(),
            mock_info(vesting_contract.as_ref(), &[]),
            owner.to_string(),
        );

        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: vesting_contract.into_string(),
            })
        );

        // "normal" unbonding succeeds and unbonding event is pushed to the pending epoch events
        let res = try_remove_mixnode(test.deps_mut(), env.clone(), info.clone());
        assert!(res.is_ok());
        let mut pending_events = interval_storage::PENDING_EPOCH_EVENTS
            .range(test.deps().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(pending_events.len(), 1);
        let event = pending_events.pop().unwrap();
        assert_eq!(1, event.0);
        assert_eq!(
            PendingEpochEventKind::UnbondMixnode { mix_id },
            event.1.kind
        );

        // but fails if repeated (since the node is already in the "unbonding" state)(
        let res = try_remove_mixnode(test.deps_mut(), env, info);
        assert_eq!(res, Err(MixnetContractError::MixnodeIsUnbonding { mix_id }))
    }

    #[test]
    fn mixnode_remove_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());

        let res = try_remove_mixnode_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &[]),
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract,
            }
        )
    }

    #[test]
    fn mixnode_remove_is_not_allowed_if_there_are_pending_pledge_changes() {
        let mut test = TestSetup::new();
        let env = test.env();

        // prior increase
        let owner = "mix-owner1";
        test.add_dummy_mixnode(owner, None);
        let sender = mock_info(owner, &[test.coin(1000)]);
        try_increase_pledge(test.deps_mut(), env.clone(), sender.clone()).unwrap();

        let res = try_remove_mixnode(test.deps_mut(), env.clone(), sender);
        assert_eq!(
            res,
            Err(MixnetContractError::PendingPledgeChange {
                pending_event_id: 1
            })
        );

        // prior decrease
        let owner = "mix-owner2";
        test.add_dummy_mixnode(owner, Some(Uint128::new(10000000000)));
        let sender = mock_info(owner, &[]);
        let amount = test.coin(1000);
        try_decrease_pledge(test.deps_mut(), env.clone(), sender, amount).unwrap();

        let sender = mock_info(owner, &[test.coin(1000)]);
        let res = try_remove_mixnode(test.deps_mut(), env.clone(), sender);
        assert_eq!(
            res,
            Err(MixnetContractError::PendingPledgeChange {
                pending_event_id: 2
            })
        );

        // artificial event
        let owner = "mix-owner3";
        let mix_id = test.add_dummy_mixnode(owner, None);
        let pending_change = PendingMixNodeChanges {
            pledge_change: Some(1234),
        };
        storage::PENDING_MIXNODE_CHANGES
            .save(test.deps_mut().storage, mix_id, &pending_change)
            .unwrap();

        let sender = mock_info(owner, &[test.coin(1000)]);
        let res = try_remove_mixnode(test.deps_mut(), env, sender);
        assert_eq!(
            res,
            Err(MixnetContractError::PendingPledgeChange {
                pending_event_id: 1234
            })
        );
    }

    #[test]
    fn updating_mixnode_config() {
        let mut test = TestSetup::new();
        let env = test.env();

        let owner = "alice";
        let info = mock_info(owner, &[]);
        let update = MixNodeConfigUpdate {
            host: "1.1.1.1:1234".to_string(),
            mix_port: 1234,
            verloc_port: 1235,
            http_api_port: 1236,
            version: "v1.2.3".to_string(),
        };

        // try updating a non existing mixnode bond
        let res = try_update_mixnode_config(test.deps_mut(), info.clone(), update.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(owner)
            })
        );

        let mix_id = test.add_dummy_mixnode(owner, None);
        let vesting_contract = test.vesting_contract();

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_update_mixnode_config_on_behalf(
            test.deps_mut(),
            mock_info(vesting_contract.as_ref(), &[]),
            update.clone(),
            owner.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: vesting_contract.into_string(),
            })
        );
        // "normal" update succeeds
        let res = try_update_mixnode_config(test.deps_mut(), info.clone(), update.clone());
        assert!(res.is_ok());

        // and the config has actually been updated
        let mix =
            must_get_mixnode_bond_by_owner(test.deps().storage, &Addr::unchecked(owner)).unwrap();
        assert_eq!(mix.mix_node.host, update.host);
        assert_eq!(mix.mix_node.mix_port, update.mix_port);
        assert_eq!(mix.mix_node.verloc_port, update.verloc_port);
        assert_eq!(mix.mix_node.http_api_port, update.http_api_port);
        assert_eq!(mix.mix_node.version, update.version);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(test.deps_mut(), env, info.clone()).unwrap();
        let res = try_update_mixnode_config(test.deps_mut(), info, update);
        assert_eq!(res, Err(MixnetContractError::MixnodeIsUnbonding { mix_id }))
    }

    #[test]
    fn updating_mixnode_config_with_illegal_proxy() {
        let mut test = TestSetup::new();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());
        let update = MixNodeConfigUpdate {
            host: "1.1.1.1:1234".to_string(),
            mix_port: 1234,
            verloc_port: 1235,
            http_api_port: 1236,
            version: "v1.2.3".to_string(),
        };

        let res = try_update_mixnode_config_on_behalf(
            test.deps_mut(),
            mock_info(illegal_proxy.as_ref(), &[]),
            update,
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract,
            }
        )
    }

    #[test]
    fn mixnode_cost_params_cant_be_updated_when_epoch_transition_is_in_progress() {
        let bad_states = vec![
            EpochState::Rewarding {
                last_rewarded: 0,
                final_node_id: 0,
            },
            EpochState::ReconcilingEvents,
            EpochState::AdvancingEpoch,
        ];

        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        for bad_state in bad_states {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "alice";
            let info = mock_info(owner, &[]);

            test.add_dummy_mixnode(owner, None);

            let mut status = EpochStatus::new(test.rewarding_validator().sender);
            status.state = bad_state;
            interval_storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

            let res =
                try_update_mixnode_cost_params(test.deps_mut(), env.clone(), info, update.clone());
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochAdvancementInProgress { .. })
            ));
        }
    }

    #[test]
    fn updating_mixnode_cost_params() {
        let mut test = TestSetup::new();
        let env = test.env();

        let owner = "alice";
        let info = mock_info(owner, &[]);
        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        // try updating a non existing mixnode bond
        let res = try_update_mixnode_cost_params(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            update.clone(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(owner)
            })
        );

        let mix_id = test.add_dummy_mixnode(owner, None);
        let vesting_contract = test.vesting_contract();

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_update_mixnode_cost_params_on_behalf(
            test.deps_mut(),
            env.clone(),
            mock_info(vesting_contract.as_ref(), &[]),
            update.clone(),
            owner.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: vesting_contract.into_string(),
            })
        );
        // "normal" update succeeds
        let res = try_update_mixnode_cost_params(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            update.clone(),
        );
        assert!(res.is_ok());

        // see if the event has been pushed onto the queue
        let mut pending_events = interval_storage::PENDING_INTERVAL_EVENTS
            .range(test.deps().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(pending_events.len(), 1);
        let event = pending_events.pop().unwrap();
        assert_eq!(1, event.0);
        assert_eq!(
            PendingIntervalEventKind::ChangeMixCostParams {
                mix_id,
                new_costs: update.clone(),
            },
            event.1.kind
        );

        // execute the event
        test_helpers::execute_all_pending_events(test.deps_mut(), env.clone());

        // and see if the config has actually been updated
        let mix = get_mixnode_details_by_id(test.deps().storage, mix_id)
            .unwrap()
            .unwrap();
        assert_eq!(mix.rewarding_details.cost_params, update);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(test.deps_mut(), env.clone(), info.clone()).unwrap();
        let res = try_update_mixnode_cost_params(test.deps_mut(), env, info, update);
        assert_eq!(res, Err(MixnetContractError::MixnodeIsUnbonding { mix_id }))
    }

    #[test]
    fn updating_mixnode_cost_params_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());
        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        let res = try_update_mixnode_cost_params_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &[]),
            update,
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract,
            }
        )
    }

    #[test]
    fn adding_mixnode_with_duplicate_sphinx_key_errors_out() {
        let mut test = TestSetup::new();
        let env = test.env();

        let keypair1 = nym_crypto::asymmetric::identity::KeyPair::new(&mut test.rng);
        let keypair2 = nym_crypto::asymmetric::identity::KeyPair::new(&mut test.rng);

        let cost_params = fixtures::mix_node_cost_params_fixture();
        let mixnode1 = MixNode {
            host: "1.2.3.4".to_string(),
            mix_port: 1234,
            verloc_port: 1234,
            http_api_port: 1234,
            sphinx_key: nym_crypto::asymmetric::encryption::KeyPair::new(&mut test.rng)
                .public_key()
                .to_base58_string(),
            identity_key: keypair1.public_key().to_base58_string(),
            version: "v0.1.2.3".to_string(),
        };

        // change identity but reuse sphinx key
        let mut mixnode2 = mixnode1.clone();
        mixnode2.sphinx_key = nym_crypto::asymmetric::encryption::KeyPair::new(&mut test.rng)
            .public_key()
            .to_base58_string();

        let sig1 =
            test.mixnode_bonding_signature(keypair1.private_key(), "alice", mixnode1.clone(), None);
        let sig2 =
            test.mixnode_bonding_signature(keypair2.private_key(), "bob", mixnode2.clone(), None);

        let info_alice = mock_info("alice", &tests::fixtures::good_mixnode_pledge());
        let info_bob = mock_info("bob", &tests::fixtures::good_mixnode_pledge());

        assert!(try_add_mixnode(
            test.deps_mut(),
            env.clone(),
            info_alice,
            mixnode1,
            cost_params.clone(),
            sig1,
        )
        .is_ok());

        // change identity but reuse sphinx key
        assert!(
            try_add_mixnode(test.deps_mut(), env, info_bob, mixnode2, cost_params, sig2).is_err()
        );
    }

    #[cfg(test)]
    mod increasing_mixnode_pledge {
        use crate::mixnodes::helpers::tests::{
            setup_mix_combinations, OWNER_UNBONDED, OWNER_UNBONDED_LEFTOVER, OWNER_UNBONDING,
        };

        use super::*;

        #[test]
        fn cant_be_performed_if_epoch_transition_is_in_progress() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let env = test.env();
                let owner = "mix-owner";

                test.add_dummy_mixnode(owner, None);

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let sender = mock_info(owner, &[test.coin(1000)]);
                let res = try_increase_pledge(test.deps_mut(), env, sender);

                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn is_not_allowed_if_account_doesnt_own_mixnode() {
            let mut test = TestSetup::new();
            let env = test.env();
            let sender = mock_info("not-mix-owner", &[]);

            let res = try_increase_pledge(test.deps_mut(), env, sender);
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: Addr::unchecked("not-mix-owner")
                })
            )
        }

        #[test]
        fn is_not_allowed_if_theres_proxy_mismatch() {
            let mut test = TestSetup::new();
            let env = test.env();

            let owner_without_proxy = Addr::unchecked("no-proxy");
            let owner_with_proxy = Addr::unchecked("with-proxy");
            let proxy = Addr::unchecked("proxy");
            let wrong_proxy = Addr::unchecked("unrelated-proxy");

            test.add_dummy_mixnode(owner_without_proxy.as_str(), None);
            test.add_dummy_mixnode_with_illegal_proxy(
                owner_with_proxy.as_str(),
                None,
                proxy.clone(),
            );

            let res = _try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                Vec::new(),
                owner_without_proxy.clone(),
                Some(proxy),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "None".to_string(),
                    incoming: "proxy".to_string(),
                })
            );

            let res = _try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                Vec::new(),
                owner_with_proxy.clone(),
                None,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "proxy".to_string(),
                    incoming: "None".to_string(),
                })
            );

            let res = _try_increase_pledge(
                test.deps_mut(),
                env,
                Vec::new(),
                owner_with_proxy.clone(),
                Some(wrong_proxy),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "proxy".to_string(),
                    incoming: "unrelated-proxy".to_string(),
                })
            )
        }

        #[test]
        fn is_not_allowed_if_mixnode_has_unbonded_or_is_unbonding() {
            let mut test = TestSetup::new();
            let env = test.env();

            // TODO: I dislike this cross-test access, but it provides us with exactly what we need
            // perhaps it should be refactored a bit?
            let owner_unbonding = Addr::unchecked(OWNER_UNBONDING);
            let owner_unbonded = Addr::unchecked(OWNER_UNBONDED);
            let owner_unbonded_leftover = Addr::unchecked(OWNER_UNBONDED_LEFTOVER);

            let ids = setup_mix_combinations(&mut test, None);
            let mix_id_unbonding = ids[1].mix_id;

            let res = try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                mock_info(owner_unbonding.as_str(), &[]),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: mix_id_unbonding
                })
            );

            // if the nodes are gone we treat them as tey never existed in the first place
            // (regardless of if there's some leftover data)
            let res = try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                mock_info(owner_unbonded_leftover.as_str(), &[]),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: owner_unbonded_leftover
                })
            );

            let res = try_increase_pledge(
                test.deps_mut(),
                env,
                mock_info(owner_unbonded.as_str(), &[]),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: owner_unbonded
                })
            )
        }

        #[test]
        fn is_not_allowed_if_no_tokens_were_sent() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";

            test.add_dummy_mixnode(owner, None);

            let sender_empty = mock_info(owner, &[]);
            let res = try_increase_pledge(test.deps_mut(), env.clone(), sender_empty);
            assert_eq!(res, Err(MixnetContractError::NoBondFound));

            let sender_zero = mock_info(owner, &[test.coin(0)]);
            let res = try_increase_pledge(test.deps_mut(), env, sender_zero);
            assert_eq!(
                res,
                Err(MixnetContractError::InsufficientPledge {
                    received: test.coin(0),
                    minimum: test.coin(1),
                })
            )
        }

        #[test]
        fn is_not_allowed_if_there_are_pending_pledge_changes() {
            let mut test = TestSetup::new();
            let env = test.env();

            // prior increase
            let owner = "mix-owner1";
            test.add_dummy_mixnode(owner, None);
            let sender = mock_info(owner, &[test.coin(1000)]);
            try_increase_pledge(test.deps_mut(), env.clone(), sender.clone()).unwrap();

            let res = try_increase_pledge(test.deps_mut(), env.clone(), sender);
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 1
                })
            );

            // prior decrease
            let owner = "mix-owner2";
            test.add_dummy_mixnode(owner, Some(Uint128::new(10000000000)));
            let sender = mock_info(owner, &[]);
            let amount = test.coin(1000);
            try_decrease_pledge(test.deps_mut(), env.clone(), sender, amount).unwrap();

            let sender = mock_info(owner, &[test.coin(1000)]);
            let res = try_increase_pledge(test.deps_mut(), env.clone(), sender);
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 2
                })
            );

            // artificial event
            let owner = "mix-owner3";
            let mix_id = test.add_dummy_mixnode(owner, None);
            let pending_change = PendingMixNodeChanges {
                pledge_change: Some(1234),
            };
            storage::PENDING_MIXNODE_CHANGES
                .save(test.deps_mut().storage, mix_id, &pending_change)
                .unwrap();

            let sender = mock_info(owner, &[test.coin(1000)]);
            let res = try_increase_pledge(test.deps_mut(), env, sender);
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 1234
                })
            );
        }

        #[test]
        fn with_valid_information_creates_pending_event() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, None);

            let events = test.pending_epoch_events();
            assert!(events.is_empty());

            let sender = mock_info(owner, &[test.coin(1000)]);
            try_increase_pledge(test.deps_mut(), env, sender).unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0].kind,
                PendingEpochEventKind::PledgeMore {
                    mix_id,
                    amount: test.coin(1000),
                }
            );
        }

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();
            let env = test.env();

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let owner = "alice";

            test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());

            let res = try_increase_pledge_on_behalf(
                test.deps_mut(),
                env,
                mock_info(illegal_proxy.as_ref(), &[coin(123, TEST_COIN_DENOM)]),
                owner.to_string(),
            )
            .unwrap_err();

            assert_eq!(
                res,
                MixnetContractError::SenderIsNotVestingContract {
                    received: illegal_proxy,
                    vesting_contract,
                }
            )
        }
    }

    #[cfg(test)]
    mod decreasing_mixnode_pledge {
        use crate::mixnodes::helpers::tests::{
            setup_mix_combinations, OWNER_UNBONDED, OWNER_UNBONDED_LEFTOVER, OWNER_UNBONDING,
        };

        use super::*;

        #[test]
        fn cant_be_performed_if_epoch_transition_is_in_progress() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let env = test.env();
                let owner = "mix-owner";
                let decrease = test.coin(1000);

                test.add_dummy_mixnode(owner, Some(Uint128::new(100_000_000_000)));

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let sender = mock_info(owner, &[]);
                let res = try_decrease_pledge(test.deps_mut(), env, sender, decrease);

                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn is_not_allowed_if_account_doesnt_own_mixnode() {
            let mut test = TestSetup::new();
            let env = test.env();
            let sender = mock_info("not-mix-owner", &[]);
            let decrease = test.coin(1000);

            let res = try_decrease_pledge(test.deps_mut(), env, sender, decrease);
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: Addr::unchecked("not-mix-owner")
                })
            )
        }

        #[test]
        fn is_not_allowed_if_theres_proxy_mismatch() {
            let mut test = TestSetup::new();
            let env = test.env();

            let owner_without_proxy = Addr::unchecked("no-proxy");
            let owner_with_proxy = Addr::unchecked("with-proxy");
            let proxy = Addr::unchecked("proxy");
            let wrong_proxy = Addr::unchecked("unrelated-proxy");

            // just to make sure that after decrease the value would still be above the minimum
            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(1000);

            test.add_dummy_mixnode(owner_without_proxy.as_str(), Some(stake));
            test.add_dummy_mixnode_with_illegal_proxy(
                owner_with_proxy.as_str(),
                Some(stake),
                proxy.clone(),
            );

            let res = _try_decrease_pledge(
                test.deps_mut(),
                env.clone(),
                decrease.clone(),
                owner_without_proxy.clone(),
                Some(proxy),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "None".to_string(),
                    incoming: "proxy".to_string(),
                })
            );

            let res = _try_decrease_pledge(
                test.deps_mut(),
                env.clone(),
                decrease.clone(),
                owner_with_proxy.clone(),
                None,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "proxy".to_string(),
                    incoming: "None".to_string(),
                })
            );

            let res = _try_decrease_pledge(
                test.deps_mut(),
                env,
                decrease,
                owner_with_proxy.clone(),
                Some(wrong_proxy),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "proxy".to_string(),
                    incoming: "unrelated-proxy".to_string(),
                })
            )
        }

        #[test]
        fn is_not_allowed_if_mixnode_has_unbonded_or_is_unbonding() {
            let mut test = TestSetup::new();
            let env = test.env();

            // just to make sure that after decrease the value would still be above the minimum
            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(1000);

            // TODO: I dislike this cross-test access, but it provides us with exactly what we need
            // perhaps it should be refactored a bit?
            let owner_unbonding = Addr::unchecked(OWNER_UNBONDING);
            let owner_unbonded = Addr::unchecked(OWNER_UNBONDED);
            let owner_unbonded_leftover = Addr::unchecked(OWNER_UNBONDED_LEFTOVER);

            let ids = setup_mix_combinations(&mut test, Some(stake));
            let mix_id_unbonding = ids[1].mix_id;

            let res = try_decrease_pledge(
                test.deps_mut(),
                env.clone(),
                mock_info(owner_unbonding.as_str(), &[]),
                decrease.clone(),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: mix_id_unbonding
                })
            );

            // if the nodes are gone we treat them as tey never existed in the first place
            // (regardless of if there's some leftover data)
            let res = try_decrease_pledge(
                test.deps_mut(),
                env.clone(),
                mock_info(owner_unbonded_leftover.as_str(), &[]),
                decrease.clone(),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: owner_unbonded_leftover
                })
            );

            let res = try_decrease_pledge(
                test.deps_mut(),
                env,
                mock_info(owner_unbonded.as_str(), &[]),
                decrease,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: owner_unbonded
                })
            )
        }

        #[test]
        fn is_not_allowed_if_it_would_result_going_below_minimum_pledge() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";

            let minimum_pledge = minimum_mixnode_pledge(test.deps().storage).unwrap();
            let pledge_amount = minimum_pledge.amount + Uint128::new(100);
            let pledged = test.coin(pledge_amount.u128());
            test.add_dummy_mixnode(owner, Some(pledge_amount));

            let invalid_decrease = test.coin(150);
            let valid_decrease = test.coin(50);

            let sender = mock_info(owner, &[]);
            let res = try_decrease_pledge(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                invalid_decrease.clone(),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::InvalidPledgeReduction {
                    current: pledged.amount,
                    decrease_by: invalid_decrease.amount,
                    minimum: minimum_pledge.amount,
                    denom: minimum_pledge.denom,
                })
            );

            let res = try_decrease_pledge(test.deps_mut(), env, sender, valid_decrease);
            assert!(res.is_ok())
        }

        #[test]
        fn provided_amount_has_to_be_nonzero() {
            let mut test = TestSetup::new();
            let env = test.env();

            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(0);

            let owner = "mix-owner";
            test.add_dummy_mixnode(owner, Some(stake));

            let sender = mock_info(owner, &[]);
            let res = try_decrease_pledge(test.deps_mut(), env, sender, decrease);
            assert_eq!(res, Err(MixnetContractError::ZeroCoinAmount))
        }

        #[test]
        fn is_not_allowed_if_there_are_pending_pledge_changes() {
            let mut test = TestSetup::new();
            let env = test.env();
            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(1000);

            // prior increase
            let owner = "mix-owner1";
            test.add_dummy_mixnode(owner, Some(stake));
            let sender = mock_info(owner, &[test.coin(1000)]);
            try_increase_pledge(test.deps_mut(), env.clone(), sender.clone()).unwrap();

            let res = try_decrease_pledge(test.deps_mut(), env.clone(), sender, decrease.clone());
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 1
                })
            );

            // prior decrease
            let owner = "mix-owner2";
            test.add_dummy_mixnode(owner, Some(stake));
            let sender = mock_info(owner, &[]);
            let amount = test.coin(1000);
            try_decrease_pledge(test.deps_mut(), env.clone(), sender, amount).unwrap();

            let sender = mock_info(owner, &[test.coin(1000)]);
            let res = try_decrease_pledge(test.deps_mut(), env.clone(), sender, decrease.clone());
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 2
                })
            );

            // artificial event
            let owner = "mix-owner3";
            let mix_id = test.add_dummy_mixnode(owner, Some(stake));
            let pending_change = PendingMixNodeChanges {
                pledge_change: Some(1234),
            };
            storage::PENDING_MIXNODE_CHANGES
                .save(test.deps_mut().storage, mix_id, &pending_change)
                .unwrap();

            let sender = mock_info(owner, &[test.coin(1000)]);
            let res = try_decrease_pledge(test.deps_mut(), env, sender, decrease);
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 1234
                })
            );
        }

        #[test]
        fn with_valid_information_creates_pending_event() {
            let mut test = TestSetup::new();
            let env = test.env();

            // just to make sure that after decrease the value would still be above the minimum
            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(1000);

            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, Some(stake));

            let events = test.pending_epoch_events();
            assert!(events.is_empty());

            let sender = mock_info(owner, &[]);
            try_decrease_pledge(test.deps_mut(), env, sender, decrease.clone()).unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0].kind,
                PendingEpochEventKind::DecreasePledge {
                    mix_id,
                    decrease_by: decrease,
                }
            );
        }

        #[test]
        fn fails_for_illegal_proxy() {
            let mut test = TestSetup::new();
            let env = test.env();

            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(1000);

            let illegal_proxy = Addr::unchecked("not-vesting-contract");
            let vesting_contract = test.vesting_contract();

            let owner = "alice";

            test.add_dummy_mixnode_with_illegal_proxy(owner, Some(stake), illegal_proxy.clone());

            let res = try_decrease_pledge_on_behalf(
                test.deps_mut(),
                env,
                mock_info(illegal_proxy.as_ref(), &[coin(123, TEST_COIN_DENOM)]),
                decrease,
                owner.to_string(),
            )
            .unwrap_err();

            assert_eq!(
                res,
                MixnetContractError::SenderIsNotVestingContract {
                    received: illegal_proxy,
                    vesting_contract,
                }
            )
        }
    }
}
