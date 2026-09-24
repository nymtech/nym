// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{MixnetEpochConfigScore, NodeChainCapability, NymNode};
use anyhow::Context;
use nym_config_score::{ConfigScoreCalculator, NodeConfigInputs};
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, PagedMixnetQueryClient};
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use nym_validator_client::nyxd::{Coin, CosmWasmCoin};
use std::collections::HashMap;
use tracing::info;

/// Materialises config score - a snapshot of each bonded node's current standing - filed under the
/// epoch in progress.
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

    /// Materialises config score for `mixnet_epoch`, the epoch in progress.
    ///
    /// Unlike the probe aggregates, config score is a snapshot of current node state with no window to
    /// replay, so it is never backfilled: a past epoch would only ever get today's state. It is filed
    /// under the current epoch going forward, and is idempotent, so a repeated pass is a no-op.
    ///
    /// A node is scored only once its inputs are actually available. The set is the bonded-node
    /// registry, so an unbonded node - or one the refresher has not yet reached - is simply absent, and
    /// a described node whose on-chain standing has not been cached yet is deferred rather than scored
    /// from a guess. An untrue score is therefore never written, and so can never later be submitted.
    pub(crate) async fn materialise(&self, mixnet_epoch: EpochId) -> anyhow::Result<()> {
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

        let nodes = self.storage.get_bonded_nym_nodes().await?;
        let capabilities: HashMap<i64, NodeChainCapability> = self
            .storage
            .get_all_node_chain_capabilities()
            .await?
            .into_iter()
            .map(|capability| (capability.node_id, capability))
            .collect();

        let (scores, deferred) =
            compute_config_scores(&calculator, mixnet_epoch, nodes, &capabilities);

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

/// Turns the bonded-node registry and the cached chain standing into config-score rows for
/// `mixnet_epoch`, returning the rows plus how many nodes were deferred. Split out from the chain and
/// storage I/O so the scoring rules - the describe/capability deferral, the decomposition, and the
/// threshold applied at score time - can be unit-tested without a chain or a database.
///
/// A described node whose on-chain standing is not yet cached is deferred (absent from the result)
/// rather than scored from a guess, so an untrue value is never produced. A node that never described
/// is scored as unavailable (a hard zero), and one with no on-chain address is scored with no balance
/// (so it takes the chain-interaction penalty), both of which are known states rather than gaps.
fn compute_config_scores(
    calculator: &ConfigScoreCalculator,
    mixnet_epoch: EpochId,
    nodes: Vec<NymNode>,
    capabilities: &HashMap<i64, NodeChainCapability>,
) -> (Vec<MixnetEpochConfigScore>, usize) {
    let mut scores = Vec::new();
    let mut deferred = 0;
    for node in nodes {
        let node = node.inner;
        let self_described_available = node.reported_version.is_some();
        let capability = capabilities.get(&node.node_id);

        // a described node advertising an address whose standing has not been cached yet has not been
        // queried: defer it rather than score it from a guess. A node that never described, or one that
        // reports no address, is not deferred - its inability to transact is a known state.
        if self_described_available && node.declared_chain_address.is_some() && capability.is_none()
        {
            deferred += 1;
            continue;
        }

        let reported_version = node
            .reported_version
            .as_deref()
            .and_then(|version| version.parse::<semver::Version>().ok());
        let runs_nym_node_binary = node.binary_name.as_deref() == Some("nym-node");
        let accepted_terms_and_conditions = node.accepted_terms_and_conditions.unwrap_or(false);
        let balance = capability
            .and_then(|c| c.balance.parse::<Coin>().ok())
            .map(CosmWasmCoin::from);
        let is_feegrant_grantee = capability.map(|c| c.is_feegrant_grantee).unwrap_or(false);

        let outcome = calculator.score(&NodeConfigInputs {
            reported_version: reported_version.as_ref(),
            runs_nym_node: runs_nym_node_binary,
            accepted_terms: accepted_terms_and_conditions,
            balance: balance.as_ref(),
            is_feegrant_grantee,
        });

        scores.push(MixnetEpochConfigScore {
            mixnet_epoch: mixnet_epoch as i64,
            node_id: node.node_id,
            score: outcome.score,
            versions_behind: outcome.versions_behind.map(i64::from),
            accepted_terms_and_conditions,
            runs_nym_node_binary,
            self_described_available,
            has_sufficient_tokens: calculator.has_sufficient_tokens(balance.as_ref()),
            is_feegrant_grantee,
        });
    }
    (scores, deferred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::node_with_ips;
    use cosmwasm_std::Uint128;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::{
        ConfigScoreParams, HistoricalNymNodeVersion, HistoricalNymNodeVersionEntry,
        OutdatedVersionWeights, VersionScoreFormulaParams,
    };
    use time::OffsetDateTime;

    const MIXNET_EPOCH: EpochId = 7;
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

    /// A fully described, up-to-date, nym-node node with terms accepted; `address` is its on-chain
    /// address, if any.
    fn described_node(node_id: i64, address: Option<&str>) -> NymNode {
        let mut inner = node_with_ips(node_id, &format!("key_{node_id}"), "1.2.3.4");
        inner.reported_version = Some("1.1.0".to_string());
        inner.binary_name = Some("nym-node".to_string());
        inner.accepted_terms_and_conditions = Some(true);
        inner.declared_chain_address = address.map(|a| a.to_string());
        NymNode { inner }
    }

    fn capability(node_id: i64, balance: u128, feegrant: bool) -> NodeChainCapability {
        NodeChainCapability {
            node_id,
            balance: Coin::new(balance, DENOM).to_string(),
            is_feegrant_grantee: feegrant,
            refreshed_at: OffsetDateTime::now_utc(),
            next_refresh_due_at: OffsetDateTime::now_utc(),
        }
    }

    fn caps(entries: Vec<NodeChainCapability>) -> HashMap<i64, NodeChainCapability> {
        entries.into_iter().map(|c| (c.node_id, c)).collect()
    }

    #[test]
    fn a_compliant_transacting_node_scores_one() {
        let (scores, deferred) = compute_config_scores(
            &calculator(1_000_000),
            MIXNET_EPOCH,
            vec![described_node(1, Some("n1abc"))],
            &caps(vec![capability(1, 1_000_000, false)]),
        );
        assert_eq!(deferred, 0);
        assert_eq!(scores.len(), 1);
        let score = &scores[0];
        assert_eq!(score.node_id, 1);
        assert_eq!(score.mixnet_epoch, MIXNET_EPOCH as i64);
        assert_eq!(score.score, 1.0);
        assert_eq!(score.versions_behind, Some(0));
        assert!(score.self_described_available);
        assert!(score.runs_nym_node_binary);
        assert!(score.accepted_terms_and_conditions);
        assert!(score.has_sufficient_tokens);
    }

    #[test]
    fn a_node_that_never_described_is_scored_unavailable() {
        // the `node_with_ips` helper leaves every config-input column unset, i.e. a bond-only node
        let (scores, deferred) = compute_config_scores(
            &calculator(1_000_000),
            MIXNET_EPOCH,
            vec![NymNode {
                inner: node_with_ips(1, "key_1", "1.2.3.4"),
            }],
            &caps(vec![]),
        );
        assert_eq!(deferred, 0);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].score, 0.0);
        assert!(!scores[0].self_described_available);
        assert_eq!(scores[0].versions_behind, None);
    }

    #[test]
    fn a_described_node_whose_standing_is_not_cached_is_deferred() {
        // described and advertising an address, but no cached capability: not queried yet, so deferred
        let (scores, deferred) = compute_config_scores(
            &calculator(1_000_000),
            MIXNET_EPOCH,
            vec![described_node(1, Some("n1abc"))],
            &caps(vec![]),
        );
        assert_eq!(deferred, 1);
        assert!(scores.is_empty());
    }

    #[test]
    fn a_described_node_with_no_address_is_penalised_not_deferred() {
        // no address means nothing to query - a known inability to transact, not a gap
        let (scores, deferred) = compute_config_scores(
            &calculator(1_000_000),
            MIXNET_EPOCH,
            vec![described_node(1, None)],
            &caps(vec![]),
        );
        assert_eq!(deferred, 0);
        assert_eq!(scores.len(), 1);
        assert!(!scores[0].has_sufficient_tokens);
        assert!(!scores[0].is_feegrant_grantee);
        // an otherwise-perfect score of 1.0 penalised by (1 - 0.2)
        assert!((scores[0].score - 0.8).abs() < 1e-9);
    }

    #[test]
    fn the_minimum_balance_is_applied_at_score_time() {
        // the same cached balance counts as sufficient or not depending only on the threshold in
        // force, with no re-query
        let capabilities = caps(vec![capability(1, 1_500_000, false)]);

        let (below, _) = compute_config_scores(
            &calculator(1_000_000),
            MIXNET_EPOCH,
            vec![described_node(1, Some("n1abc"))],
            &capabilities,
        );
        assert!(below[0].has_sufficient_tokens);
        assert_eq!(below[0].score, 1.0);

        let (above, _) = compute_config_scores(
            &calculator(2_000_000),
            MIXNET_EPOCH,
            vec![described_node(1, Some("n1abc"))],
            &capabilities,
        );
        assert!(!above[0].has_sufficient_tokens);
        assert!((above[0].score - 0.8).abs() < 1e-9);
    }
}
