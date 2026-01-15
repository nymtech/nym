// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::TestedNodeDetails;
use anyhow::{Context, anyhow, bail};
use nym_authenticator_requests::AuthenticatorVersion;
use nym_http_api_client::UserAgent;
use nym_sdk::mixnet::NodeIdentity;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::NymNodeDescription;
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use tracing::{debug, info};
use url::Url;

// in the old behaviour we were getting all skimmed nodes to retrieve performance
// that was ultimately unused
// should we want to use it again, the code is commented out below
//
// #[derive(Clone)]
// pub struct DescribedNodeWithPerformance {
//     pub(crate) described: NymNodeDescription,
//     // in old behaviour there was no filtering here,
//     // but in case that ever changes, this value is available
//     pub(crate) performance: u8,
// }
//
// impl DescribedNodeWithPerformance {
//     pub fn identity(&self) -> NodeIdentity {
//         self.described.ed25519_identity_key()
//     }
//
//     pub fn to_testable_node(&self) -> anyhow::Result<TestedNodeDetails> {
//         let exit_router_address = self
//             .described
//             .description
//             .ip_packet_router
//             .as_ref()
//             .map(|ipr| ipr.address.parse().context("malformed ipr address"))
//             .transpose()?;
//         let authenticator_address = self
//             .described
//             .description
//             .authenticator
//             .as_ref()
//             .map(|ipr| {
//                 ipr.address
//                     .parse()
//                     .context("malformed authenticator address")
//             })
//             .transpose()?;
//         let authenticator_version = AuthenticatorVersion::from(
//             self.described
//                 .description
//                 .build_information
//                 .build_version
//                 .as_str(),
//         );
//         let ip_address = self
//             .described
//             .description
//             .host_information
//             .ip_address
//             .first()
//             .copied();
//
//         Ok(TestedNodeDetails {
//             identity: self.identity(),
//             exit_router_address,
//             authenticator_address,
//             authenticator_version,
//             ip_address,
//         })
//     }
// }

#[derive(Clone)]
pub struct DirectoryNode {
    described: NymNodeDescription,
}

impl DirectoryNode {
    pub fn identity(&self) -> NodeIdentity {
        self.described.ed25519_identity_key()
    }

    pub fn to_testable_node(&self) -> anyhow::Result<TestedNodeDetails> {
        let exit_router_address = self
            .described
            .description
            .ip_packet_router
            .as_ref()
            .map(|ipr| ipr.address.parse().context("malformed ipr address"))
            .transpose()?;
        let authenticator_address = self
            .described
            .description
            .authenticator
            .as_ref()
            .map(|ipr| {
                ipr.address
                    .parse()
                    .context("malformed authenticator address")
            })
            .transpose()?;
        let authenticator_version = AuthenticatorVersion::from(
            self.described
                .description
                .build_information
                .build_version
                .as_str(),
        );
        let ip_address = self
            .described
            .description
            .host_information
            .ip_address
            .first()
            .copied();

        Ok(TestedNodeDetails {
            identity: self.identity(),
            exit_router_address,
            authenticator_address,
            authenticator_version,
            ip_address,
        })
    }
}

pub struct NymApiDirectory {
    // nodes: HashMap<NodeIdentity, DescribedNodeWithPerformance>,
    nodes: HashMap<NodeIdentity, DirectoryNode>,
}

impl NymApiDirectory {
    // obtain all needed directory information on genesis
    pub async fn new(api_url: Url) -> anyhow::Result<Self> {
        let user_agent: UserAgent = nym_bin_common::bin_info_local_vergen!().into();
        let api_client = nym_http_api_client::Client::builder(api_url)
            .context("malformed nym api url")?
            .with_user_agent(user_agent)
            .build()
            .context("failed to build nym api client")?;

        debug!("Fetching all described nodes from nym-api...");
        let described_nodes = api_client
            .get_all_described_nodes()
            .await
            .context("nym api query failure")?;

        // let skimmed_nodes = api_client
        //     .get_all_basic_nodes_with_metadata()
        //     .await
        //     .context("nym api query failure")?;
        //
        // let performances = skimmed_nodes
        //     .nodes
        //     .into_iter()
        //     .map(|n| (n.node_id, n.performance))
        //     .collect::<HashMap<_, _>>();
        //
        // let mut nodes = HashMap::new();
        // for described_node in described_nodes {
        //     let identity = described_node.ed25519_identity_key();
        //     let Some(performance) = performances.get(&described_node.node_id) else {
        //         tracing::warn!(
        //             "Failed to append mixnet_performance, node {identity} not found among the skimmed nodes",
        //         );
        //         continue;
        //     };
        //     let info = DescribedNodeWithPerformance {
        //         described: described_node,
        //         performance: performance.round_to_integer(),
        //     };
        //     nodes.insert(identity, info);
        // }

        let nodes = described_nodes
            .into_iter()
            .map(|described| {
                (
                    described.ed25519_identity_key(),
                    DirectoryNode { described },
                )
            })
            .collect();

        Ok(NymApiDirectory { nodes })
    }

    pub fn random_exit_with_ipr(&self) -> anyhow::Result<NodeIdentity> {
        info!("Selecting random gateway with IPR enabled");
        self.nodes
            .iter()
            .filter(|(_, n)| n.described.description.ip_packet_router.is_some())
            .choose(&mut rand::thread_rng())
            .ok_or(anyhow!("no gateways running IPR available"))
            .map(|(id, _)| *id)
    }

    pub fn random_entry_gateway(&self) -> anyhow::Result<NodeIdentity> {
        info!("Selecting random entry gateway");
        self.nodes
            .iter()
            .filter(|(_, n)| n.described.description.declared_role.entry)
            .choose(&mut rand::thread_rng())
            .ok_or(anyhow!("no entry gateways available"))
            .map(|(id, _)| *id)
    }

    pub fn get_nym_node(&self, identity: NodeIdentity) -> anyhow::Result<DirectoryNode> {
        self.nodes
            .get(&identity)
            .cloned()
            .ok_or_else(|| anyhow!("did not find node {identity}"))
    }

    pub fn entry_gateway(&self, identity: &NodeIdentity) -> anyhow::Result<DirectoryNode> {
        let Some(maybe_entry) = self.nodes.get(identity).cloned() else {
            bail!("{identity} does not exist")
        };
        if !maybe_entry.described.description.declared_role.entry {
            bail!("{identity} is not an entry node")
        };
        Ok(maybe_entry)
    }
}
