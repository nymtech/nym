// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use nym_api_requests::models::{ConfigScore, NymNodeDescription};
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

pub(crate) fn calculate_config_score(
    config_score_data: &ConfigScoreData,
    described_data: Option<&NymNodeDescription>,
) -> ConfigScore {
    let Some(described) = described_data else {
        return ConfigScore::unavailable();
    };

    let node_version = &described.description.build_information.build_version;
    let Ok(reported_semver) = node_version.parse::<semver::Version>() else {
        return ConfigScore::bad_semver();
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

    ConfigScore::new(
        version_score,
        versions_behind,
        accepted_terms_and_conditions,
        runs_nym_node,
    )
}
