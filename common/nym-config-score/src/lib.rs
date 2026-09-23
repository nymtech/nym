// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared config-score computation, so that nym-api and the network monitor orchestrator
//! produce an identical score from identical inputs.

use cosmwasm_std::Coin;
use nym_contracts_common::NaiveFloat;
use nym_mixnet_contract_common::{
    ConfigScoreParams, HistoricalNymNodeVersionEntry, VersionScoreFormulaParams,
};

/// Per-node inputs to a config-score computation.
pub struct NodeConfigInputs<'a> {
    /// Reported semver, or `None` when there is no self-description or it does not parse.
    pub reported_version: Option<&'a semver::Version>,

    /// Whether the node runs the `nym-node` binary.
    pub runs_nym_node: bool,

    /// Whether the operator accepted the terms and conditions.
    pub accepted_terms: bool,

    /// The node's on-chain balance, or `None` when unknown.
    pub balance: Option<&'a Coin>,

    /// Whether the node is a feegrant grantee.
    pub is_feegrant_grantee: bool,
}

/// The outcome of a config-score computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfigScoreOutcome {
    /// The config score in `[0, 1]`.
    pub score: f64,

    /// Weighted versions behind the on-chain head, or `None` when no version was scored.
    pub versions_behind: Option<u32>,
}

/// Computes config scores against a fixed policy and version history, bound once per pass.
pub struct ConfigScoreCalculator {
    minimum_balance: Coin,
    chain_interactions_penalty: f64,
    config_score_params: ConfigScoreParams,
    version_history: Vec<HistoricalNymNodeVersionEntry>,
}

impl ConfigScoreCalculator {
    pub fn new(
        minimum_balance: Coin,
        chain_interactions_penalty: f64,
        config_score_params: ConfigScoreParams,
        version_history: Vec<HistoricalNymNodeVersionEntry>,
    ) -> Self {
        ConfigScoreCalculator {
            minimum_balance,
            chain_interactions_penalty,
            config_score_params,
            version_history,
        }
    }

    /// Whether the balance meets the configured minimum, in the same denom.
    pub fn has_sufficient_tokens(&self, balance: Option<&Coin>) -> bool {
        let Some(balance) = balance else {
            return false;
        };
        // we always query in the minimum's denom; never score sufficiency across denoms
        if balance.denom != self.minimum_balance.denom {
            return false;
        }
        balance.amount >= self.minimum_balance.amount
    }

    /// Computes the config score for a single node.
    pub fn score(&self, node: &NodeConfigInputs) -> ConfigScoreOutcome {
        let Some(reported_version) = node.reported_version else {
            return ConfigScoreOutcome {
                score: 0.0,
                versions_behind: None,
            };
        };

        let versions_behind = self
            .config_score_params
            .version_weights
            .versions_behind_factor(reported_version, &self.version_history);

        // the binary and terms gates are hard zeros
        let mut score = if !node.runs_nym_node || !node.accepted_terms {
            0.0
        } else {
            version_score(
                versions_behind,
                self.config_score_params.version_score_formula_params,
            )
        };

        // an inability to transact on chain is a soft penalty
        let can_send_transactions =
            self.has_sufficient_tokens(node.balance) || node.is_feegrant_grantee;
        if !can_send_transactions {
            score *= 1.0 - self.chain_interactions_penalty;
        }

        ConfigScoreOutcome {
            score,
            versions_behind: Some(versions_behind),
        }
    }
}

/// version_score = penalty ^ (versions_behind ^ penalty_scaling)
fn version_score(versions_behind: u32, params: VersionScoreFormulaParams) -> f64 {
    let penalty = params.penalty.naive_to_f64();
    let scaling = params.penalty_scaling.naive_to_f64();
    penalty.powf((versions_behind as f64).powf(scaling))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Uint128;
    use nym_mixnet_contract_common::{
        HistoricalNymNodeVersion, HistoricalNymNodeVersionEntry, OutdatedVersionWeights,
    };

    const DENOM: &str = "unym";
    const MINIMUM: u128 = 1_000_000;

    fn coin(amount: u128) -> Coin {
        Coin {
            denom: DENOM.to_string(),
            amount: Uint128::new(amount),
        }
    }

    fn version(v: &str) -> semver::Version {
        v.parse().unwrap()
    }

    // release chain: genesis 1.0.0 -> head 1.1.0, one minor apart
    fn history() -> Vec<HistoricalNymNodeVersionEntry> {
        let genesis = HistoricalNymNodeVersion::genesis("1.0.0".to_string(), 1);
        let head_diff = genesis.cumulative_difference_since_genesis(&version("1.1.0"));
        let head = HistoricalNymNodeVersion {
            semver: "1.1.0".to_string(),
            introduced_at_height: 2,
            difference_since_genesis: head_diff,
        };
        vec![
            HistoricalNymNodeVersionEntry {
                id: 0,
                version_information: genesis,
            },
            HistoricalNymNodeVersionEntry {
                id: 1,
                version_information: head,
            },
        ]
    }

    fn calculator() -> ConfigScoreCalculator {
        ConfigScoreCalculator::new(
            coin(MINIMUM),
            0.2,
            ConfigScoreParams {
                version_weights: OutdatedVersionWeights::default(),
                version_score_formula_params: VersionScoreFormulaParams::default(),
            },
            history(),
        )
    }

    #[test]
    fn latest_compliant_transacting_node_scores_one() {
        let calc = calculator();
        let head = version("1.1.0");
        let balance = coin(MINIMUM);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&head),
            runs_nym_node: true,
            accepted_terms: true,
            balance: Some(&balance),
            is_feegrant_grantee: false,
        });
        assert_eq!(outcome.versions_behind, Some(0));
        assert_eq!(outcome.score, 1.0);
    }

    #[test]
    fn unaccepted_terms_zero_the_score() {
        let calc = calculator();
        let head = version("1.1.0");
        let balance = coin(MINIMUM);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&head),
            runs_nym_node: true,
            accepted_terms: false,
            balance: Some(&balance),
            is_feegrant_grantee: false,
        });
        assert_eq!(outcome.score, 0.0);
    }

    #[test]
    fn non_nym_node_binary_zeroes_the_score() {
        let calc = calculator();
        let head = version("1.1.0");
        let balance = coin(MINIMUM);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&head),
            runs_nym_node: false,
            accepted_terms: true,
            balance: Some(&balance),
            is_feegrant_grantee: false,
        });
        assert_eq!(outcome.score, 0.0);
    }

    #[test]
    fn missing_version_scores_zero_and_reports_no_distance() {
        let calc = calculator();
        let balance = coin(MINIMUM);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: None,
            runs_nym_node: true,
            accepted_terms: true,
            balance: Some(&balance),
            is_feegrant_grantee: false,
        });
        assert_eq!(outcome.score, 0.0);
        assert_eq!(outcome.versions_behind, None);
    }

    #[test]
    fn being_behind_reduces_the_score() {
        let calc = calculator();
        let behind = version("1.0.0");
        let balance = coin(MINIMUM);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&behind),
            runs_nym_node: true,
            accepted_terms: true,
            balance: Some(&balance),
            is_feegrant_grantee: false,
        });
        // one minor behind, default minor weight 10
        assert_eq!(outcome.versions_behind, Some(10));
        // 0.995 ^ (10 ^ 1.65) ~= 0.7994
        assert!(outcome.score < 1.0);
        assert!((outcome.score - 0.7994).abs() < 1e-3);
    }

    #[test]
    fn inability_to_transact_applies_the_soft_penalty() {
        let calc = calculator();
        let head = version("1.1.0");
        let poor = coin(MINIMUM - 1);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&head),
            runs_nym_node: true,
            accepted_terms: true,
            balance: Some(&poor),
            is_feegrant_grantee: false,
        });
        // full score 1.0 penalised by (1 - 0.2)
        assert!((outcome.score - 0.8).abs() < 1e-9);
    }

    #[test]
    fn a_feegrant_avoids_the_penalty_despite_low_balance() {
        let calc = calculator();
        let head = version("1.1.0");
        let poor = coin(0);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&head),
            runs_nym_node: true,
            accepted_terms: true,
            balance: Some(&poor),
            is_feegrant_grantee: true,
        });
        assert_eq!(outcome.score, 1.0);
    }

    #[test]
    fn sufficient_balance_avoids_the_penalty() {
        let calc = calculator();
        let head = version("1.1.0");
        let exact = coin(MINIMUM);
        let outcome = calc.score(&NodeConfigInputs {
            reported_version: Some(&head),
            runs_nym_node: true,
            accepted_terms: true,
            balance: Some(&exact),
            is_feegrant_grantee: false,
        });
        assert_eq!(outcome.score, 1.0);
    }

    #[test]
    fn has_sufficient_tokens_rules() {
        let calc = calculator();
        assert!(!calc.has_sufficient_tokens(None));
        assert!(!calc.has_sufficient_tokens(Some(&coin(MINIMUM - 1))));
        assert!(calc.has_sufficient_tokens(Some(&coin(MINIMUM))));
        assert!(calc.has_sufficient_tokens(Some(&coin(MINIMUM + 1))));
        // a different denom never counts, whatever the amount
        let other_denom = Coin {
            denom: "unyx".to_string(),
            amount: Uint128::new(MINIMUM * 1000),
        };
        assert!(!calc.has_sufficient_tokens(Some(&other_denom)));
    }
}
