// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::array::TryFromSliceError;

use crate::MixLayer;
use nym_sphinx_addressing::NodeIdentity;
use nym_sphinx_types::NymPacketError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymTopologyError {
    #[error("the provided network topology is empty - there are no valid nodes on it - the network request(s) probably failed")]
    EmptyNetworkTopology,

    #[error("no node with identity {node_identity} is known")]
    NonExistentNode { node_identity: Box<NodeIdentity> },

    #[error("could not use node with identity {node_identity} as egress since it didn't get assigned valid role in the current epoch")]
    InvalidEgressRole { node_identity: Box<NodeIdentity> },

    #[error("one (or more) of mixing layers does not have any valid nodes available")]
    InsufficientMixingNodes,

    #[error("The provided network topology has no gateways available")]
    NoGatewaysAvailable,

    #[error("The provided network topology has no valid mixnodes available")]
    NoMixnodesAvailable,

    #[error("Gateway with identity key {identity_key} doesn't exist")]
    NonExistentGatewayError { identity_key: String },

    #[error("timed out while waiting for gateway '{identity_key}' to come online")]
    TimedOutWaitingForGateway { identity_key: String },

    #[error("Wanted to create a mix route with {requested} hops, while only {available} layers are available")]
    InvalidNumberOfHopsError { available: usize, requested: usize },

    #[error("No mixnodes available on layer {layer}")]
    EmptyMixLayer { layer: MixLayer },

    #[error("Uneven layer distribution. Layer {layer} has {nodes} on it, while we expected a value between {lower_bound} and {upper_bound} as we have {total_nodes} nodes in total. Full breakdown: {layer_distribution:?}")]
    UnevenLayerDistribution {
        layer: MixLayer,
        nodes: usize,
        lower_bound: usize,
        upper_bound: usize,
        total_nodes: usize,
        layer_distribution: Vec<(MixLayer, usize)>,
    },
    // We can't import SurbAckRecoveryError due to cyclic dependency, this is a bit dirty
    #[error("Could not build payload")]
    PayloadBuilder,

    #[error("Outfox: {0}")]
    Outfox(#[from] nym_sphinx_types::OutfoxError),

    #[error("{0}")]
    FromSlice(#[from] TryFromSliceError),

    #[error("{0}")]
    PacketError(#[from] NymPacketError),

    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("{0}")]
    VarError(#[from] std::env::VarError),
}
