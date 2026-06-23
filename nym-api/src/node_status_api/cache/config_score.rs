// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use cosmwasm_std::Coin;
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::{
    ChainInteractionCapabilities, ChainInteractionCapabilitiesDetailed, ConfigScoreV2,
};
use nym_contracts_common::NaiveFloat;
use nym_mixnet_contract_common::VersionScoreFormulaParams;

fn versions_behind_factor_to_config_score(
    versions_behind: u32,
    params: VersionScoreFormulaParams,
) -> f64 {
    let penalty = params.penalty.naive_to_f64();
    let scaling = params.penalty_scaling.naive_to_f64();

    // version_score = penalty ^ (num_versions_behind ^ penalty_scaling)
    penalty.powf((versions_behind as f64).powf(scaling))
}

fn has_sufficient_tokens(
    minimum_balance: &Coin,
    capabilities: &Option<ChainInteractionCapabilitiesDetailed>,
) -> bool {
    let Some(capabilities) = capabilities else {
        return false;
    };
    let chain_balance = &capabilities.on_chain_balance;

    // this should never happen because we have queried for this specific balance,
    // but some defensive coding never hurt
    if chain_balance.denom != minimum_balance.denom {
        return false;
    }
    chain_balance.amount >= minimum_balance.amount
}

pub(crate) fn calculate_config_score(
    minimum_balance: &Coin,
    config_score_data: &ConfigScoreData,
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
    let versions_behind = config_score_data
        .config_score_params
        .version_weights
        .versions_behind_factor(
            &reported_semver,
            &config_score_data.nym_node_version_history,
        );

    let runs_nym_node = described.description.build_information.binary_name == "nym-node";
    let accepted_terms_and_conditions = described
        .description
        .auxiliary_details
        .accepted_operator_terms_and_conditions;

    let version_score = if !runs_nym_node || !accepted_terms_and_conditions {
        0.
    } else {
        versions_behind_factor_to_config_score(
            versions_behind,
            config_score_data
                .config_score_params
                .version_score_formula_params,
        )
    };

    let chain_interaction = ChainInteractionCapabilities {
        has_sufficient_tokens: has_sufficient_tokens(minimum_balance, chain_capabilities),
        is_fee_grant_grantee: chain_capabilities
            .as_ref()
            .map(|c| c.is_feegrant_grantee)
            .unwrap_or_default(),
    };

    ConfigScoreV2::new(
        version_score,
        versions_behind,
        accepted_terms_and_conditions,
        runs_nym_node,
        chain_interaction,
    )
}
