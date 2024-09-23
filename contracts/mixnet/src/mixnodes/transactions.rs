// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::compat::helpers::{
    ensure_can_decrease_pledge, ensure_can_increase_pledge, ensure_can_modify_cost_params,
};
use crate::interval::storage as interval_storage;
use crate::interval::storage::push_new_interval_event;
use crate::mixnodes::helpers::{get_mixnode_details_by_owner, must_get_mixnode_bond_by_owner};
use crate::nodes::storage as nymnodes_storage;
use crate::nodes::transactions::add_nym_node_inner;
use crate::support::helpers::{
    ensure_bonded, ensure_epoch_in_progress_state, ensure_no_pending_params_changes,
    ensure_no_pending_pledge_changes, validate_pledge,
};
use cosmwasm_std::{coin, Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_migrated_mixnode_event, new_mixnode_config_update_event,
    new_pending_cost_params_update_event, new_pending_mixnode_unbonding_event,
    new_pending_pledge_decrease_event, new_pending_pledge_increase_event,
};
use mixnet_contract_common::mixnode::{MixNodeConfigUpdate, NodeCostParams};
use mixnet_contract_common::pending_events::{PendingEpochEventKind, PendingIntervalEventKind};
use mixnet_contract_common::{
    MixNode, MixNodeDetails, MixnodeBondingPayload, NymNodeBond, PendingNodeChanges,
};
use nym_contracts_common::signing::MessageSignature;

pub fn try_add_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
    cost_params: NodeCostParams,
    owner_signature: MessageSignature,
) -> Result<Response, MixnetContractError> {
    let signed_payload = MixnodeBondingPayload::new(mix_node.clone(), cost_params.clone());

    // any mixnode added via 'BondMixnode' endpoint should get added as a NymNode
    add_nym_node_inner(
        deps,
        env,
        info,
        mix_node.into(),
        cost_params,
        owner_signature,
        signed_payload,
    )
}

pub fn try_increase_mixnode_pledge(
    deps: DepsMut<'_>,
    env: Env,
    increase: Vec<Coin>,
    mix_details: MixNodeDetails,
) -> Result<Response, MixnetContractError> {
    let mut pending_changes = mix_details.pending_changes;
    let mix_id = mix_details.mix_id();

    ensure_can_increase_pledge(deps.storage, &mix_details)?;

    let rewarding_denom = &mix_details.original_pledge().denom;
    let pledge_increase = validate_pledge(increase, coin(1, rewarding_denom))?;

    let cosmos_event = new_pending_pledge_increase_event(mix_id, &pledge_increase);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::MixnodePledgeMore {
        mix_id,
        amount: pledge_increase,
    };
    let epoch_event_id = interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;
    pending_changes.pledge_change = Some(epoch_event_id);
    storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub fn try_decrease_mixnode_pledge(
    deps: DepsMut<'_>,
    env: Env,
    decrease_by: Coin,
    mix_details: MixNodeDetails,
) -> Result<Response, MixnetContractError> {
    let mut pending_changes = mix_details.pending_changes;
    let mix_id = mix_details.mix_id();

    ensure_can_decrease_pledge(deps.storage, &mix_details, &decrease_by)?;

    let cosmos_event = new_pending_pledge_decrease_event(mix_id, &decrease_by);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::MixnodeDecreasePledge {
        mix_id,
        decrease_by,
    };
    let epoch_event_id = interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;
    pending_changes.pledge_change = Some(epoch_event_id);
    storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn try_remove_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &info.sender)?;
    let pending_changes = storage::PENDING_MIXNODE_CHANGES
        .may_load(deps.storage, existing_bond.mix_id)?
        .unwrap_or_default();

    // unbonding is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    // see if the proxy matches
    ensure_bonded(&existing_bond)?;

    // if there are any pending requests to change the pledge, wait for them to resolve before allowing the unbonding
    ensure_no_pending_pledge_changes(&pending_changes)?;

    // set `is_unbonding` field
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
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &info.sender)?;

    ensure_bonded(&existing_bond)?;

    let cfg_update_event =
        new_mixnode_config_update_event(existing_bond.mix_id, &info.sender, &new_config);

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
    deps: DepsMut,
    env: Env,
    new_costs: NodeCostParams,
    mix_details: MixNodeDetails,
) -> Result<Response, MixnetContractError> {
    let mut pending_changes = mix_details.pending_changes;
    let mix_id = mix_details.mix_id();

    ensure_can_modify_cost_params(deps.storage, &mix_details)?;

    let cosmos_event = new_pending_cost_params_update_event(mix_id, &new_costs);

    // push the interval event
    let interval_event = PendingIntervalEventKind::ChangeMixCostParams { mix_id, new_costs };
    let interval_event_id = push_new_interval_event(deps.storage, &env, interval_event)?;
    pending_changes.cost_params_change = Some(interval_event_id);
    storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub fn try_migrate_to_nymnode(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    let mix_details = get_mixnode_details_by_owner(deps.storage, info.sender.clone())?.ok_or(
        MixnetContractError::NoAssociatedMixNodeBond {
            owner: info.sender.clone(),
        },
    )?;
    let node_id = mix_details.mix_id();
    let pending_changes = mix_details.pending_changes;
    let mixnode_bond = mix_details.bond_information;

    if mixnode_bond.proxy.is_some() {
        return Err(MixnetContractError::VestingNodeMigration);
    }

    ensure_epoch_in_progress_state(deps.storage)?;
    ensure_no_pending_pledge_changes(&pending_changes)?;
    ensure_no_pending_params_changes(&pending_changes)?;
    ensure_bonded(&mixnode_bond)?;

    let mixnode_identity = mixnode_bond.mix_node.identity_key.clone();

    // remove mixnode bond data
    storage::mixnode_bonds().replace(deps.storage, node_id, None, Some(&mixnode_bond))?;

    // NOTE: nothing happens to rewarding data as its structure hasn't changed, and it's accessible under `node_id` key

    // create nym-node entry
    // note: since the starting value of nymnode counter was the same one as the final value of mixnode counter,
    // we know there's definitely nothing under this key saved.
    let nym_node_bond = NymNodeBond::new(
        node_id,
        mixnode_bond.owner,
        mixnode_bond.original_pledge,
        mixnode_bond.mix_node,
        mixnode_bond.bonding_height,
    );
    nymnodes_storage::nym_nodes().save(deps.storage, node_id, &nym_node_bond)?;

    // move pending changes
    // TODO: what if node has pending PM change?
    storage::PENDING_MIXNODE_CHANGES.remove(deps.storage, node_id);
    nymnodes_storage::PENDING_NYMNODE_CHANGES.save(
        deps.storage,
        node_id,
        &PendingNodeChanges::new_empty(),
    )?;

    Ok(Response::new().add_event(new_migrated_mixnode_event(
        &info.sender,
        &mixnode_identity,
        node_id,
    )))
}
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::compat::transactions::try_increase_pledge;
    use crate::contract::execute;
    use crate::mixnet_contract_settings::storage::minimum_node_pledge;
    use crate::mixnodes::helpers::{get_mixnode_details_by_id, get_mixnode_details_by_identity};
    use crate::nodes::helpers::{get_node_details_by_identity, must_get_node_bond_by_owner};
    use crate::signing::storage as signing_storage;
    use crate::support::tests::fixtures::{good_mixnode_pledge, TEST_COIN_DENOM};
    use crate::support::tests::test_helpers::TestSetup;
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{Addr, Order, StdResult, Uint128};
    use mixnet_contract_common::mixnode::PendingMixNodeChanges;
    use mixnet_contract_common::nym_node::Role;
    use mixnet_contract_common::{EpochState, EpochStatus, ExecuteMsg, Percent};

    #[test]
    fn mixnode_add() -> anyhow::Result<()> {
        let mut test = TestSetup::new();
        let env = test.env();

        let sender = "alice";
        let minimum_pledge = minimum_node_pledge(test.deps().storage).unwrap();
        let mut insufficient_pledge = minimum_pledge.clone();
        insufficient_pledge.amount -= Uint128::new(1000);

        // if we don't send enough funds
        let info = mock_info(sender, &[insufficient_pledge.clone()]);
        let (mixnode, sig, _) =
            test.mixnode_with_signature(sender, Some(vec![insufficient_pledge.clone()]));
        let cost_params = fixtures::node_cost_params_fixture();

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
        test.add_legacy_mixnode(sender, None);

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

        test.add_legacy_gateway(sender2, None);

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

        let result = try_add_mixnode(
            test.deps_mut(),
            env,
            info.clone(),
            mixnode.clone(),
            cost_params,
            sig,
        );
        assert!(result.is_ok());

        // and the node has been added as a nym-node
        let nym_node =
            get_node_details_by_identity(test.deps().storage, mixnode.identity_key.clone())
                .unwrap()
                .unwrap();
        assert_eq!(nym_node.bond_information.owner, info.sender);

        let maybe_legacy =
            get_mixnode_details_by_identity(test.deps().storage, mixnode.identity_key)?;
        assert!(maybe_legacy.is_none());

        // make sure we got assigned the next id (note: we have already bonded a mixnode and a gateway before in this test)
        let bond =
            must_get_node_bond_by_owner(test.deps().storage, &Addr::unchecked(sender2)).unwrap();
        assert_eq!(3, bond.node_id);

        Ok(())
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
        let cost_params = fixtures::node_cost_params_fixture();

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

        test.immediately_unbond_node(1);
        let res = try_add_mixnode(test.deps_mut(), env, info, mixnode, cost_params, signature);
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));
    }

    #[test]
    fn removing_mixnode_cant_be_performed_if_epoch_transition_is_in_progress() {
        let bad_states = vec![
            EpochState::Rewarding {
                last_rewarded: 0,
                final_node_id: 0,
            },
            EpochState::ReconcilingEvents,
            EpochState::RoleAssignment {
                next: Role::first(),
            },
        ];

        for bad_state in bad_states {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "alice";
            let info = mock_info(owner, &[]);

            test.add_legacy_mixnode(owner, None);

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

        let mix_id = test.add_legacy_mixnode(owner, None);

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
    fn mixnode_remove_is_not_allowed_if_there_are_pending_pledge_changes() {
        let mut test = TestSetup::new();
        let env = test.env();

        // prior increase
        let owner = "mix-owner1";
        let node_id = test.add_legacy_mixnode(owner, None);
        let details = test.mixnode_by_id(node_id).unwrap();

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
        let node_id = test.add_legacy_mixnode(owner, Some(Uint128::new(10000000000)));
        let details = test.mixnode_by_id(node_id).unwrap();
        let amount = test.coin(1000);
        try_decrease_mixnode_pledge(test.deps_mut(), env.clone(), amount, details).unwrap();

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
        let mix_id = test.add_legacy_mixnode(owner, None);
        let pending_change = PendingMixNodeChanges {
            pledge_change: Some(1234),
            cost_params_change: None,
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

        let mix_id = test.add_legacy_mixnode(owner, None);

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
    fn mixnode_cost_params_cant_be_updated_when_epoch_transition_is_in_progress() {
        let bad_states = vec![
            EpochState::Rewarding {
                last_rewarded: 0,
                final_node_id: 0,
            },
            EpochState::ReconcilingEvents,
            EpochState::RoleAssignment {
                next: Role::first(),
            },
        ];

        let update = NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        for bad_state in bad_states {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "alice";

            let node_id = test.add_legacy_mixnode(owner, None);
            let details = test.mixnode_by_id(node_id).unwrap();

            let mut status = EpochStatus::new(test.rewarding_validator().sender);
            status.state = bad_state;
            interval_storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

            let res = try_update_mixnode_cost_params(
                test.deps_mut(),
                env.clone(),
                update.clone(),
                details,
            );
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
        let update = NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        let node_id = test.add_legacy_mixnode(owner, None);
        let details = test.mixnode_by_id(node_id).unwrap();

        // "normal" update succeeds
        let res = try_update_mixnode_cost_params(
            test.deps_mut(),
            env.clone(),
            update.clone(),
            details.clone(),
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
                mix_id: node_id,
                new_costs: update.clone(),
            },
            event.1.kind
        );

        // execute the event
        test_helpers::execute_all_pending_events(test.deps_mut(), env.clone());

        // and see if the config has actually been updated
        let mix = get_mixnode_details_by_id(test.deps().storage, node_id)
            .unwrap()
            .unwrap();
        assert_eq!(mix.rewarding_details.cost_params, update);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(test.deps_mut(), env.clone(), info.clone()).unwrap();
        let details = test.mixnode_by_id(node_id).unwrap();
        let res = try_update_mixnode_cost_params(test.deps_mut(), env, update, details);
        assert_eq!(res, Err(MixnetContractError::NodeIsUnbonding { node_id }))
    }

    #[test]
    fn adding_mixnode_with_duplicate_sphinx_key_errors_out() {
        let mut test = TestSetup::new();
        let env = test.env();

        let keypair1 = nym_crypto::asymmetric::identity::KeyPair::new(&mut test.rng);
        let keypair2 = nym_crypto::asymmetric::identity::KeyPair::new(&mut test.rng);

        let cost_params = fixtures::node_cost_params_fixture();
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
                EpochState::RoleAssignment {
                    next: Role::first(),
                },
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let env = test.env();
                let owner = "mix-owner";

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let node_id = test.add_legacy_mixnode(owner, None);
                let details = test.mixnode_by_id(node_id).unwrap();
                let increase = test.coins(1000);
                let res = try_increase_mixnode_pledge(test.deps_mut(), env, increase, details);

                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn is_not_allowed_if_mixnode_has_unbonded() {
            let mut test = TestSetup::new();
            let env = test.env();

            let ids = setup_mix_combinations(&mut test, None);
            let mix_id_unbonding = ids[1].mix_id;

            let increase = test.coins(1000);
            let details = test.mixnode_by_id(mix_id_unbonding).unwrap();

            let res = try_increase_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                increase.clone(),
                details,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NodeIsUnbonding {
                    node_id: mix_id_unbonding
                })
            );
        }

        #[test]
        fn is_not_allowed_if_no_tokens_were_sent() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";

            let node_id = test.add_legacy_mixnode(owner, None);
            let details = test.mixnode_by_id(node_id).unwrap();

            let sender_empty = Vec::new();
            let res = try_increase_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                sender_empty,
                details.clone(),
            );
            assert_eq!(res, Err(MixnetContractError::NoBondFound));

            let sender_zero = test.coins(0);
            let res =
                try_increase_mixnode_pledge(test.deps_mut(), env, sender_zero, details.clone());
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
            let node_id = test.add_legacy_mixnode(owner, None);
            let details = test.mixnode_by_id(node_id).unwrap();
            let sender = test.coins(1000);
            try_increase_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                details.clone(),
            )
            .unwrap();

            let details = test.mixnode_by_id(node_id).unwrap();
            let res = try_increase_mixnode_pledge(test.deps_mut(), env.clone(), sender, details);
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 1
                })
            );

            // prior decrease
            let owner = "mix-owner2";
            let node_id = test.add_legacy_mixnode(owner, Some(Uint128::new(10000000000)));
            let details = test.mixnode_by_id(node_id).unwrap();

            let amount = test.coin(1000);
            try_decrease_mixnode_pledge(test.deps_mut(), env.clone(), amount, details.clone())
                .unwrap();

            let sender = test.coins(10000);
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = try_increase_mixnode_pledge(test.deps_mut(), env.clone(), sender, details);
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 2
                })
            );
        }

        #[test]
        fn with_valid_information_creates_pending_event() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";
            let mix_id = test.add_legacy_mixnode(owner, None);
            let details = test.mixnode_by_id(mix_id).unwrap();

            let events = test.pending_epoch_events();
            assert!(events.is_empty());

            let sender = test.coins(1000);
            try_increase_mixnode_pledge(test.deps_mut(), env, sender, details).unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0].kind,
                PendingEpochEventKind::MixnodePledgeMore {
                    mix_id,
                    amount: test.coin(1000),
                }
            );
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
                EpochState::RoleAssignment {
                    next: Role::first(),
                },
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let env = test.env();
                let owner = "mix-owner";
                let decrease = test.coin(1000);

                let node_id = test.add_legacy_mixnode(owner, Some(Uint128::new(100_000_000_000)));
                let details = test.mixnode_by_id(node_id).unwrap();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let res = try_decrease_mixnode_pledge(test.deps_mut(), env, decrease, details);

                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn is_not_allowed_if_mixnode_is_unbonding() {
            let mut test = TestSetup::new();
            let env = test.env();

            let ids = setup_mix_combinations(&mut test, None);
            let mix_id_unbonding = ids[1].mix_id;

            let decrease = test.coin(1000);
            let details = test.mixnode_by_id(mix_id_unbonding).unwrap();

            let res = try_decrease_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                decrease.clone(),
                details,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NodeIsUnbonding {
                    node_id: mix_id_unbonding
                })
            );
        }

        #[test]
        fn is_not_allowed_if_it_would_result_going_below_minimum_pledge() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";

            let minimum_pledge = minimum_node_pledge(test.deps().storage).unwrap();
            let pledge_amount = minimum_pledge.amount + Uint128::new(100);
            let pledged = test.coin(pledge_amount.u128());
            let node_id = test.add_legacy_mixnode(owner, Some(pledge_amount));
            let details = test.mixnode_by_id(node_id).unwrap();

            let invalid_decrease = test.coin(150);
            let valid_decrease = test.coin(50);

            let res = try_decrease_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                invalid_decrease.clone(),
                details.clone(),
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

            let res = try_decrease_mixnode_pledge(test.deps_mut(), env, valid_decrease, details);
            assert!(res.is_ok())
        }

        #[test]
        fn provided_amount_has_to_be_nonzero() {
            let mut test = TestSetup::new();
            let env = test.env();

            let stake = Uint128::new(100_000_000_000);
            let decrease = test.coin(0);

            let owner = "mix-owner";
            let node_id = test.add_legacy_mixnode(owner, Some(stake));
            let details = test.mixnode_by_id(node_id).unwrap();

            let res = try_decrease_mixnode_pledge(test.deps_mut(), env, decrease, details);
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
            let node_id = test.add_legacy_mixnode(owner, Some(stake));
            let details = test.mixnode_by_id(node_id).unwrap();

            let sender = test.coins(1000);
            try_increase_mixnode_pledge(test.deps_mut(), env.clone(), sender.clone(), details)
                .unwrap();

            let details = test.mixnode_by_id(node_id).unwrap();
            let res = try_decrease_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                decrease.clone(),
                details,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 1
                })
            );

            // prior decrease
            let owner = "mix-owner2";
            let node_id = test.add_legacy_mixnode(owner, Some(stake));
            let details = test.mixnode_by_id(node_id).unwrap();
            let amount = test.coin(1000);
            try_decrease_mixnode_pledge(test.deps_mut(), env.clone(), amount, details).unwrap();

            let details = test.mixnode_by_id(node_id).unwrap();
            let res = try_decrease_mixnode_pledge(
                test.deps_mut(),
                env.clone(),
                decrease.clone(),
                details,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::PendingPledgeChange {
                    pending_event_id: 2
                })
            );

            // artificial event
            let owner = "mix-owner3";
            let mix_id = test.add_legacy_mixnode(owner, Some(stake));
            let pending_change = PendingMixNodeChanges {
                pledge_change: Some(1234),
                cost_params_change: None,
            };
            storage::PENDING_MIXNODE_CHANGES
                .save(test.deps_mut().storage, mix_id, &pending_change)
                .unwrap();

            let details = test.mixnode_by_id(mix_id).unwrap();
            let res = try_decrease_mixnode_pledge(test.deps_mut(), env, decrease, details);
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
            let mix_id = test.add_legacy_mixnode(owner, Some(stake));
            let details = test.mixnode_by_id(mix_id).unwrap();

            let events = test.pending_epoch_events();
            assert!(events.is_empty());

            let sender = mock_info(owner, &[]);
            try_decrease_mixnode_pledge(test.deps_mut(), env, decrease.clone(), details).unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0].kind,
                PendingEpochEventKind::MixnodeDecreasePledge {
                    mix_id,
                    decrease_by: decrease,
                }
            );
        }
    }
}
