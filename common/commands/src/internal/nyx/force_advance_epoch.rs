// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use anyhow::bail;
use clap::Parser;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::NodeRewardingParameters;
use nym_mixnet_contract_common::{
    EpochRewardedSet, EpochState, NodeId, RewardingParams, RoleAssignment,
};
use nym_validator_client::nyxd::contract_traits::mixnet_query_client::MixnetQueryClientExt;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use rand::prelude::*;
use rand::thread_rng;

#[derive(Debug, Parser)]
pub struct Args {}

fn choose_new_nodes(
    params: &RewardingParams,
    rewarded_set: &EpochRewardedSet,
    role: Role,
) -> Vec<NodeId> {
    let mut rng = thread_rng();

    match role {
        Role::EntryGateway => rewarded_set
            .assignment
            .entry_gateways
            .choose_multiple(&mut rng, params.rewarded_set.entry_gateways as usize)
            .copied()
            .collect(),
        Role::Layer1 => rewarded_set
            .assignment
            .layer1
            .choose_multiple(&mut rng, params.rewarded_set.mixnodes as usize / 3)
            .copied()
            .collect(),
        Role::Layer2 => rewarded_set
            .assignment
            .layer2
            .choose_multiple(&mut rng, params.rewarded_set.mixnodes as usize / 3)
            .copied()
            .collect(),
        Role::Layer3 => rewarded_set
            .assignment
            .layer3
            .choose_multiple(&mut rng, params.rewarded_set.mixnodes as usize / 3)
            .copied()
            .collect(),
        Role::ExitGateway => rewarded_set
            .assignment
            .exit_gateways
            .choose_multiple(&mut rng, params.rewarded_set.exit_gateways as usize)
            .copied()
            .collect(),
        Role::Standby => rewarded_set
            .assignment
            .standby
            .choose_multiple(&mut rng, params.rewarded_set.standby as usize)
            .copied()
            .collect(),
    }
}

pub async fn force_advance_epoch(_: Args, client: SigningClient) -> anyhow::Result<()> {
    let current_epoch = client.get_current_interval_details().await?;
    let epoch_status = client.get_current_epoch_status().await?;
    if epoch_status.being_advanced_by.as_str() != client.address().to_string() {
        bail!(
            "this client is not authorised to perform any epoch operations. we need {}",
            client.address()
        )
    }

    let rewarding_params = client.get_rewarding_parameters().await?;
    let current_rewarded_set = client.get_rewarded_set().await?;

    if !current_epoch.is_current_epoch_over {
        println!("the current epoch is not over yet - there's nothing to do")
    }

    // is this most efficient? no. but it's simple
    loop {
        let epoch_status = client.get_current_epoch_status().await?;

        match epoch_status.state {
            EpochState::InProgress => break,
            EpochState::Rewarding { final_node_id, .. } => {
                println!("rewarding {final_node_id} with big fat 0...");
                client
                    .reward_node(
                        final_node_id,
                        NodeRewardingParameters::new(Default::default(), Default::default()),
                        None,
                    )
                    .await?;
            }
            EpochState::ReconcilingEvents => {
                println!("trying to reconcile events...");
                client.reconcile_epoch_events(None, None).await?;
            }
            EpochState::RoleAssignment { next } => {
                let nodes = choose_new_nodes(&rewarding_params, &current_rewarded_set, next);
                println!("assigning {nodes:?} as {next}");

                client
                    .assign_roles(RoleAssignment { role: next, nodes }, None)
                    .await?;
            }
        }
    }

    Ok(())
}
