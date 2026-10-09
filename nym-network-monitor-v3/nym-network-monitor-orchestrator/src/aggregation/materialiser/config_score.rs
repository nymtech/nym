// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{ConfigScoreCandidate, MixnetEpochConfigScore};
use anyhow::Context;
use nym_config_score::{ConfigScoreCalculator, NodeConfigInputs};
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, PagedMixnetQueryClient};
use nym_validator_client::nyxd::nym_mixnet_contract_common::{EpochId, Interval};
use nym_validator_client::nyxd::{Coin, CosmWasmCoin};
use time::OffsetDateTime;
use tracing::info;

/// Materialises config score - a snapshot of each described node's current standing - filed under
/// the epoch in progress.
///
/// Holds its own chain client for the contract's scoring params and nym-node version history. The
/// on-chain balance and feegrant it scores from come from the capability cache (via storage), kept
/// warm by a separate refresher, so this never queries a node directly.
pub(crate) struct ConfigScoreMaterialiser<C> {
    /// Minimum on-chain balance a node must hold to count as able to transact.
    minimum_balance: Coin,

    /// Penalty applied to a node that cannot transact on chain.
    chain_interactions_penalty: f64,

    client: C,

    storage: NetworkMonitorStorage,
}

impl<C: MixnetQueryClient + Sync> ConfigScoreMaterialiser<C> {
    pub(crate) fn new(
        minimum_balance: Coin,
        chain_interactions_penalty: f64,
        client: C,
        storage: NetworkMonitorStorage,
    ) -> Self {
        ConfigScoreMaterialiser {
            minimum_balance,
            chain_interactions_penalty,
            client,
            storage,
        }
    }

    /// Materialises config score for the epoch in progress in `interval`.
    ///
    /// Unlike the probe aggregates, config score is a snapshot of current node state with no window to
    /// replay, so it is never backfilled: a past epoch would only ever get today's state. It is filed
    /// under the current epoch going forward, and a repeated pass leaves every stored row as it was.
    ///
    /// A node is scored only once its inputs are actually available. The set is the described nodes,
    /// so an unbonded node, or one never described completely, is simply absent, and a described node
    /// whose on-chain standing has not been cached yet is deferred rather than scored from a guess.
    /// An untrue score is therefore never written, and so can never later be submitted.
    pub(crate) async fn materialise(&self, interval: &Interval) -> anyhow::Result<()> {
        let mixnet_epoch = interval.current_epoch_absolute_id();

        let config_score_params = self
            .client
            .get_mixnet_contract_state_params()
            .await
            .context("failed to query the config-score params from the mixnet contract")?
            .config_score_params;
        let version_history = self
            .client
            .get_full_nym_node_version_history()
            .await
            .context("failed to query the nym-node version history from the mixnet contract")?;

        // the shared crate scores in cosmwasm's `Coin`, matching nym-api; convert the nyxd balance at
        // this boundary
        let calculator = ConfigScoreCalculator::new(
            self.minimum_balance.clone().into(),
            self.chain_interactions_penalty,
            config_score_params,
            version_history,
        );

        let candidates = self.storage.get_config_score_candidates().await?;
        let (scores, deferred) = compute_config_scores(
            &calculator,
            mixnet_epoch,
            interval.current_epoch_start(),
            candidates,
        );

        let materialised = scores.len();
        self.storage
            .batch_insert_mixnet_epoch_config_scores(&scores)
            .await?;
        info!(
            mixnet_epoch,
            materialised, deferred, "materialised config scores"
        );
        Ok(())
    }
}

/// Turns the described nodes and their cached chain standing into config-score rows for
/// `mixnet_epoch`, which began at `epoch_start`, returning the rows plus how many nodes were
/// deferred. Split out from the chain and storage I/O so the scoring rules - the capability
/// deferral, the decomposition, and the threshold applied at score time - can be unit-tested
/// without a chain or a database.
fn compute_config_scores(
    calculator: &ConfigScoreCalculator,
    mixnet_epoch: EpochId,
    epoch_start: OffsetDateTime,
    candidates: Vec<ConfigScoreCandidate>,
) -> (Vec<MixnetEpochConfigScore>, usize) {
    let mut scores = Vec::new();
    let mut deferred = 0;
    for candidate in candidates {
        // a node whose standing has not been cached yet has not been queried: defer it rather than
        // score it from a guess
        let (Some(balance), Some(is_feegrant_grantee)) =
            (candidate.balance, candidate.is_feegrant_grantee)
        else {
            deferred += 1;
            continue;
        };

        let reported_version = candidate.reported_version.parse::<semver::Version>().ok();
        let balance = balance.parse::<Coin>().ok().map(CosmWasmCoin::from);

        let breakdown = calculator.score(&NodeConfigInputs {
            reported_version,
            runs_nym_node: candidate.binary_name == "nym-node",
            accepted_terms: candidate.accepted_terms_and_conditions,
            balance,
            is_feegrant_grantee,
        });

        scores.push(MixnetEpochConfigScore {
            mixnet_epoch: mixnet_epoch as i64,
            epoch_start,
            node_id: candidate.node_id,
            score: breakdown.score,
            versions_behind: breakdown.versions_behind.map(i64::from),
            accepted_terms_and_conditions: breakdown.accepted_terms,
            runs_nym_node_binary: breakdown.runs_nym_node,
            has_sufficient_tokens: breakdown.has_sufficient_tokens,
            is_feegrant_grantee: breakdown.is_feegrant_grantee,
        });
    }
    (scores, deferred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Uint128;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::{
        ConfigScoreParams, HistoricalNymNodeVersion, HistoricalNymNodeVersionEntry,
        OutdatedVersionWeights, VersionScoreFormulaParams,
    };
    use time::macros::datetime;

    const MIXNET_EPOCH: EpochId = 7;
    const EPOCH_START: OffsetDateTime = datetime!(2025-06-01 12:00:00 UTC);
    const DENOM: &str = "unym";

    // release chain: genesis 1.0.0 -> head 1.1.0, so a node reporting 1.1.0 is zero versions behind
    fn history() -> Vec<HistoricalNymNodeVersionEntry> {
        let genesis = HistoricalNymNodeVersion::genesis("1.0.0".to_string(), 1);
        let head_diff = genesis.cumulative_difference_since_genesis(&"1.1.0".parse().unwrap());
        vec![
            HistoricalNymNodeVersionEntry {
                id: 0,
                version_information: genesis,
            },
            HistoricalNymNodeVersionEntry {
                id: 1,
                version_information: HistoricalNymNodeVersion {
                    semver: "1.1.0".to_string(),
                    introduced_at_height: 2,
                    difference_since_genesis: head_diff,
                },
            },
        ]
    }

    fn calculator(minimum: u128) -> ConfigScoreCalculator {
        ConfigScoreCalculator::new(
            CosmWasmCoin {
                denom: DENOM.to_string(),
                amount: Uint128::new(minimum),
            },
            0.2,
            ConfigScoreParams {
                version_weights: OutdatedVersionWeights::default(),
                version_score_formula_params: VersionScoreFormulaParams::default(),
            },
            history(),
        )
    }

    /// An up-to-date nym-node with terms accepted, holding `balance` and no feegrant.
    fn candidate(node_id: i64, balance: u128) -> ConfigScoreCandidate {
        ConfigScoreCandidate {
            node_id,
            reported_version: "1.1.0".to_string(),
            binary_name: "nym-node".to_string(),
            accepted_terms_and_conditions: true,
            balance: Some(Coin::new(balance, DENOM).to_string()),
            is_feegrant_grantee: Some(false),
        }
    }

    /// [`candidate`] before its on-chain standing has been cached.
    fn uncached(node_id: i64) -> ConfigScoreCandidate {
        ConfigScoreCandidate {
            balance: None,
            is_feegrant_grantee: None,
            ..candidate(node_id, 0)
        }
    }

    fn compute(
        minimum: u128,
        candidates: Vec<ConfigScoreCandidate>,
    ) -> (Vec<MixnetEpochConfigScore>, usize) {
        compute_config_scores(&calculator(minimum), MIXNET_EPOCH, EPOCH_START, candidates)
    }

    #[test]
    fn a_compliant_transacting_node_scores_one() {
        let (scores, deferred) = compute(1_000_000, vec![candidate(1, 1_000_000)]);

        assert_eq!(deferred, 0);
        assert_eq!(scores.len(), 1);
        let score = &scores[0];
        assert_eq!(score.node_id, 1);
        assert_eq!(score.mixnet_epoch, MIXNET_EPOCH as i64);
        assert_eq!(score.epoch_start, EPOCH_START);
        assert_eq!(score.score, 1.0);
        assert_eq!(score.versions_behind, Some(0));
        assert!(score.runs_nym_node_binary);
        assert!(score.accepted_terms_and_conditions);
        assert!(score.has_sufficient_tokens);
    }

    // a reported version is free text, so one that does not parse is a known state with a known
    // answer rather than a gap
    #[test]
    fn a_version_that_does_not_parse_scores_a_hard_zero() {
        let unparseable = ConfigScoreCandidate {
            reported_version: "not-a-version".to_string(),
            ..candidate(1, 1_000_000)
        };
        let (scores, deferred) = compute(1_000_000, vec![unparseable]);

        assert_eq!(deferred, 0);
        assert_eq!(scores[0].score, 0.0);
        assert_eq!(scores[0].versions_behind, None);
    }

    #[test]
    fn a_node_whose_standing_is_not_cached_is_deferred() {
        let (scores, deferred) = compute(1_000_000, vec![uncached(1), candidate(2, 1_000_000)]);

        assert_eq!(deferred, 1);
        let scored: Vec<_> = scores.iter().map(|score| score.node_id).collect();
        assert_eq!(scored, vec![2]);
    }

    #[test]
    fn the_minimum_balance_is_applied_at_score_time() {
        // the same cached balance counts as sufficient or not depending only on the threshold in
        // force, with no re-query
        let (below, _) = compute(1_000_000, vec![candidate(1, 1_500_000)]);
        assert!(below[0].has_sufficient_tokens);
        assert_eq!(below[0].score, 1.0);

        let (above, _) = compute(2_000_000, vec![candidate(1, 1_500_000)]);
        assert!(!above[0].has_sufficient_tokens);
        assert!((above[0].score - 0.8).abs() < 1e-9);
    }
}
