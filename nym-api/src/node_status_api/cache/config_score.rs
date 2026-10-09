// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::{ChainInteractionCapabilitiesDetailed, ConfigScoreV2};
use nym_config_score::{ConfigScoreCalculator, NodeConfigInputs};

/// Pulls a node's config-score inputs out of its self-description and cached chain standing.
fn node_config_inputs(
    described: &NymNodeDescriptionV3,
    chain_capabilities: &Option<ChainInteractionCapabilitiesDetailed>,
) -> NodeConfigInputs {
    let build = &described.description.build_information;
    NodeConfigInputs {
        reported_version: build.build_version.parse().ok(),
        runs_nym_node: build.binary_name == "nym-node",
        accepted_terms: described
            .description
            .auxiliary_details
            .accepted_operator_terms_and_conditions,
        balance: chain_capabilities
            .as_ref()
            .map(|c| c.on_chain_balance.clone()),
        is_feegrant_grantee: chain_capabilities
            .as_ref()
            .map(|c| c.is_feegrant_grantee)
            .unwrap_or_default(),
    }
}

/// Assembles a node's [`ConfigScoreV2`] from its self-description and cached chain standing, scoring
/// via the shared `nym-config-score` calculator; an undescribed node has no inputs to score.
pub(crate) fn calculate_config_score(
    calculator: &ConfigScoreCalculator,
    described_data: Option<&NymNodeDescriptionV3>,
    chain_capabilities: &Option<ChainInteractionCapabilitiesDetailed>,
) -> ConfigScoreV2 {
    let Some(described) = described_data else {
        return ConfigScoreV2::unavailable();
    };
    calculator
        .score(&node_config_inputs(described, chain_capabilities))
        .into()
}
