// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::{
    ChainInteractionCapabilities, ChainInteractionCapabilitiesDetailed, ConfigScoreV2,
};
use nym_config_score::{ConfigScoreCalculator, NodeConfigInputs};

/// Assemble a node's [`ConfigScoreV2`] from its self-description and chain standing.
///
/// The scoring itself lives in `nym-config-score`, shared with the network monitor orchestrator so
/// the two produce an identical number; this is the nym-api-side adapter that pulls the inputs out
/// of the described data and wraps the outcome in the response type. The `calculator` carries the
/// per-refresh policy and version history and is built once for the whole population.
pub(crate) fn calculate_config_score(
    calculator: &ConfigScoreCalculator,
    described_data: Option<&NymNodeDescriptionV3>,
    chain_capabilities: &Option<ChainInteractionCapabilitiesDetailed>,
) -> ConfigScoreV2 {
    let Some(described) = described_data else {
        return ConfigScoreV2::unavailable();
    };

    let node_version = &described.description.build_information.build_version;
    let Ok(reported_semver) = node_version.parse::<semver::Version>() else {
        return ConfigScoreV2::bad_semver();
    };

    let runs_nym_node = described.description.build_information.binary_name == "nym-node";
    let accepted_terms_and_conditions = described
        .description
        .auxiliary_details
        .accepted_operator_terms_and_conditions;

    let balance = chain_capabilities.as_ref().map(|c| &c.on_chain_balance);
    let is_fee_grant_grantee = chain_capabilities
        .as_ref()
        .map(|c| c.is_feegrant_grantee)
        .unwrap_or_default();

    let outcome = calculator.score(&NodeConfigInputs {
        reported_version: Some(&reported_semver),
        runs_nym_node,
        accepted_terms: accepted_terms_and_conditions,
        balance,
        is_feegrant_grantee: is_fee_grant_grantee,
    });

    let chain_interaction = ChainInteractionCapabilities {
        has_sufficient_tokens: outcome.has_sufficient_tokens,
        is_fee_grant_grantee,
    };

    ConfigScoreV2::new(
        outcome.score,
        outcome.versions_behind.unwrap_or_default(),
        accepted_terms_and_conditions,
        runs_nym_node,
        chain_interaction,
    )
}
