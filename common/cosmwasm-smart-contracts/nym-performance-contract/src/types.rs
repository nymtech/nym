// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymPerformanceContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Env, Timestamp};
use nym_contracts_common::Percent;
use serde::de::{self, Deserialize, Deserializer};

pub use nym_contracts_common::digest::{DigestLeaf, DigestResponse};

pub type EpochId = u32;
pub type NodeId = u32;

/// One monitor's per-kind values for a node. Absent kinds are omitted from JSON.
#[cw_serde]
#[derive(Copy, Default)]
pub struct Measurements {
    #[serde(rename = "l", default, skip_serializing_if = "Option::is_none")]
    pub liveness: Option<Percent>,

    #[serde(rename = "s", default, skip_serializing_if = "Option::is_none")]
    pub stress: Option<Percent>,

    #[serde(rename = "c", default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Percent>,
}

impl Measurements {
    pub fn is_empty(&self) -> bool {
        self.liveness.is_none() && self.stress.is_none() && self.config.is_none()
    }

    /// Sets the liveness value.
    pub fn with_liveness(mut self, value: Percent) -> Self {
        self.liveness = Some(value);
        self
    }

    /// Sets the stress value.
    pub fn with_stress(mut self, value: Percent) -> Self {
        self.stress = Some(value);
        self
    }

    /// Sets the config value.
    pub fn with_config(mut self, value: Percent) -> Self {
        self.config = Some(value);
        self
    }
}

/// Per-kind medians of one node's bundle.
#[cw_serde]
#[derive(Copy, Default)]
pub struct KindMedians {
    pub liveness: Option<Percent>,
    pub stress: Option<Percent>,
    pub config: Option<Percent>,
}

#[cw_serde]
pub struct LastSubmission {
    pub block_height: u64,
    pub block_time: Timestamp,

    // not as relevant, but might as well store it
    pub data: Option<LastSubmittedData>,
}

#[cw_serde]
pub struct LastSubmittedData {
    pub sender: Addr,
    pub epoch_id: EpochId,
    pub data: NodeSubmission,
}

#[cw_serde]
pub struct NetworkMonitorDetails {
    pub address: Addr,
    pub authorised_by: Addr,
    pub authorised_at_height: u64,
}

impl NetworkMonitorDetails {
    pub fn retire(self, env: &Env, sender: &Addr) -> RetiredNetworkMonitor {
        RetiredNetworkMonitor {
            details: self,
            retired_by: sender.clone(),
            retired_at_height: env.block.height,
        }
    }
}

#[cw_serde]
pub struct RetiredNetworkMonitor {
    pub details: NetworkMonitorDetails,
    pub retired_by: Addr,
    pub retired_at_height: u64,
}

/// One monitor's measurements for one node: any subset of kinds, one value each.
#[cw_serde]
#[derive(Copy)]
pub struct NodeSubmission {
    #[serde(rename = "n")]
    pub node_id: NodeId,

    #[serde(rename = "m")]
    pub measurements: Measurements,
}

#[cw_serde]
pub struct NetworkMonitorSubmissionMetadata {
    pub last_submitted_epoch_id: EpochId,
    pub last_submitted_node_id: NodeId,
}

/// Converts a value to the integer percent it is stored as.
fn to_stored(value: Percent) -> u8 {
    value.round_to_two_decimal_places().round_to_integer()
}

/// Rebuilds the `Percent` an integer percent was stored from.
fn from_stored(value: u8) -> Percent {
    // SAFETY: stored values are within 0..=100, enforced by `to_stored` and `de_stored_percents`
    #[allow(clippy::unwrap_used)]
    Percent::from_percentage_value(u64::from(value)).unwrap()
}

fn de_stored_percents<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer)?;
    if values.is_empty() {
        return Err(de::Error::custom("stored results must not be empty"));
    }
    if let Some(bad) = values.iter().find(|value| **value > 100) {
        return Err(de::Error::custom(format!(
            "stored percent {bad} exceeds 100"
        )));
    }
    Ok(values)
}

/// Every monitor's value for one kind, as sorted integer percents.
#[cw_serde]
pub struct NodeResults(#[serde(deserialize_with = "de_stored_percents")] Vec<u8>);

impl NodeResults {
    pub fn new(initial: Percent) -> NodeResults {
        NodeResults(vec![to_stored(initial)])
    }

    // ASSUMPTION: number of NM will be relatively small, so loading the whole vector of values
    // to insert new one and resave is cheap
    pub fn insert_new(&mut self, result: Percent) {
        let stored = to_stored(result);
        let pos = self.0.binary_search(&stored).unwrap_or_else(|e| e);
        self.0.insert(pos, stored);
    }

    /// Feeds the score the digest commits: read [`Weights::score`] before changing it.
    // SAFETY: there are no codepaths that allow constructing empty struct
    pub fn median(&self) -> Percent {
        let len = self.0.len();
        if len % 2 == 1 {
            // odd number of elements: return the middle one
            from_stored(self.0[len / 2])
        } else {
            // even number: average the two middle elements
            let mid1 = from_stored(self.0[len / 2 - 1]);
            let mid2 = from_stored(self.0[len / 2]);
            mid1.average(&mid2).round_to_two_decimal_places()
        }
    }

    /// The stored values as percents, ascending.
    pub fn values(&self) -> impl Iterator<Item = Percent> + '_ {
        self.0.iter().copied().map(from_stored)
    }

    /// Adds a value to the results in `slot`, creating them from it when absent.
    pub fn add_to(slot: &mut Option<NodeResults>, value: Percent) {
        match slot {
            Some(existing) => existing.insert_new(value),
            None => *slot = Some(NodeResults::new(value)),
        }
    }
}

/// Everything submitted for one node in one epoch, split by kind. Absent kinds are omitted from JSON.
#[cw_serde]
pub struct EpochNodeMeasurements {
    #[serde(rename = "l", default, skip_serializing_if = "Option::is_none")]
    pub liveness: Option<NodeResults>,

    #[serde(rename = "s", default, skip_serializing_if = "Option::is_none")]
    pub stress: Option<NodeResults>,

    #[serde(rename = "c", default, skip_serializing_if = "Option::is_none")]
    pub config: Option<NodeResults>,
}

impl EpochNodeMeasurements {
    /// Starts a bundle from the first monitor's measurements.
    pub fn new(measurements: Measurements) -> Self {
        let mut bundle = EpochNodeMeasurements {
            liveness: None,
            stress: None,
            config: None,
        };
        bundle.insert(measurements);
        bundle
    }

    /// Merges one monitor's measurements, kind by kind.
    pub fn insert(&mut self, measurements: Measurements) {
        if let Some(value) = measurements.liveness {
            NodeResults::add_to(&mut self.liveness, value);
        }
        if let Some(value) = measurements.stress {
            NodeResults::add_to(&mut self.stress, value);
        }
        if let Some(value) = measurements.config {
            NodeResults::add_to(&mut self.config, value);
        }
    }

    /// The median of every kind present.
    pub fn medians(&self) -> KindMedians {
        KindMedians {
            liveness: self.liveness.as_ref().map(NodeResults::median),
            stress: self.stress.as_ref().map(NodeResults::median),
            config: self.config.as_ref().map(NodeResults::median),
        }
    }
}

/// Share of the score carried by each routing kind; zero means the kind does not contribute.
#[cw_serde]
#[derive(Copy)]
pub struct Weights {
    #[serde(default)]
    pub liveness: Percent,

    #[serde(default)]
    pub stress: Percent,
}

impl Weights {
    pub fn validate(&self) -> Result<(), NymPerformanceContractError> {
        let total = self.liveness.value() + self.stress.value();
        if total != Decimal::one() {
            return Err(NymPerformanceContractError::WeightsDoNotSumToOne { total });
        }
        Ok(())
    }

    /// nym-api's formula: the renormalised weighted mean of the applied routing kinds, times config.
    /// `None` when no weighted kind was measured or config is missing.
    ///
    /// The digest commits this score for every node's latest bundle, and the contract subtracts
    /// an entry by recomputing it, so the score of a stored bundle under its epoch's weights must
    /// never change. A plain code change therefore must not alter existing scores, which rules out:
    ///
    ///   - counting an absent kind as zero rather than renormalising it away;
    ///   - changing config's role (a threshold, a weighted term, a score without it), or adding
    ///     another required gate that older bundles lack;
    ///   - curves, caps or thresholds on the result;
    ///   - other precision or rounding;
    ///   - another median rule (see [`NodeResults::median`]), outlier rejection or a minimum
    ///     number of monitors;
    ///   - inputs beyond the bundle and these weights, such as stake, role or another contract's
    ///     state, which would break the digest even without a code change.
    ///
    /// Adding a weighted kind is fine: older bundles lack it and absent kinds are skipped.
    /// Anything above must instead be selected by a version carried in the weights, so earlier
    /// epochs keep this formula, or ship with a migration that re-folds the accumulator.
    /// `score_outputs_are_frozen` pins sample outputs so an accidental change fails.
    pub fn score(&self, medians: KindMedians) -> Option<Percent> {
        let config = medians.config?;

        let mut weighted_total = Decimal::zero();
        let mut applied_weight = Decimal::zero();
        for (weight, median) in [
            (self.liveness, medians.liveness),
            (self.stress, medians.stress),
        ] {
            // a zero weight adds nothing, so only measured kinds with a share count
            if let Some(median) = median {
                weighted_total += weight.value() * median.value();
                applied_weight += weight.value();
            }
        }

        if applied_weight.is_zero() {
            return None;
        }

        let raw = weighted_total / applied_weight * config.value();
        // SAFETY: a weighted mean of percents times a percent cannot exceed one
        #[allow(clippy::unwrap_used)]
        Some(Percent::new(raw).unwrap().round_to_two_decimal_places())
    }
}

/// Weights together with the epoch they took effect from.
#[cw_serde]
#[derive(Copy)]
pub struct EpochWeights {
    pub effective_from: EpochId,
    pub weights: Weights,
}

/// Per-kind medians of one node's bundle and their combined score under that epoch's weights.
#[cw_serde]
pub struct EpochNodePerformance {
    pub epoch_id: EpochId,
    pub medians: KindMedians,
    pub score: Option<Percent>,
}

#[cw_serde]
pub struct NodePerformanceResponse {
    pub performance: Option<EpochNodePerformance>,
}

#[cw_serde]
pub struct NodeMeasurementsResponse {
    pub measurements: Option<EpochNodeMeasurements>,
}

#[cw_serde]
pub struct NodePerformancePagedResponse {
    pub node_id: NodeId,
    pub performance: Vec<EpochNodePerformance>,
    pub start_next_after: Option<EpochId>,
}

/// One node's medians and score within an epoch page.
#[cw_serde]
pub struct NodePerformance {
    pub node_id: NodeId,
    pub medians: KindMedians,
    pub score: Option<Percent>,
}

#[cw_serde]
pub struct EpochPerformancePagedResponse {
    pub epoch_id: EpochId,
    pub performance: Vec<NodePerformance>,
    pub start_next_after: Option<NodeId>,
}

#[cw_serde]
pub struct NodeMeasurements {
    pub node_id: NodeId,
    pub measurements: EpochNodeMeasurements,
}

#[cw_serde]
pub struct EpochMeasurementsPagedResponse {
    pub epoch_id: EpochId,
    pub measurements: Vec<NodeMeasurements>,
    pub start_next_after: Option<NodeId>,
}

#[cw_serde]
pub struct HistoricalPerformance {
    pub epoch_id: EpochId,
    pub node_id: NodeId,
    pub medians: KindMedians,
    pub score: Option<Percent>,
}

#[cw_serde]
pub struct FullHistoricalPerformancePagedResponse {
    pub performance: Vec<HistoricalPerformance>,
    pub start_next_after: Option<(EpochId, NodeId)>,
}

/// A node's newest bundle as the digest commits it: its score under the weights of its own epoch.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cw_serde]
pub struct LatestPerformance {
    pub node_id: NodeId,
    pub epoch_id: EpochId,
    #[cfg_attr(feature = "utoipa", schema(value_type = Option<String>))]
    pub score: Option<Percent>,
}

impl LatestPerformance {
    /// Versions the leaf encoding: a new encoding takes a new tag.
    const LEAF_TAG: u8 = 1;

    /// The tag byte, the 4-byte node and epoch ids, the score's presence byte and its 16-byte
    /// `u128` atomics.
    const LEAF_LEN: usize = 26;

    /// The entry for `bundle`, scored under `weights` when known.
    pub fn new(
        node_id: NodeId,
        epoch_id: EpochId,
        bundle: &EpochNodeMeasurements,
        weights: Option<&Weights>,
    ) -> Self {
        LatestPerformance {
            node_id,
            epoch_id,
            score: weights.and_then(|weights| weights.score(bundle.medians())),
        }
    }
}

impl DigestLeaf for LatestPerformance {
    /// The canonical LtHash leaf: the tag `1`, `node_id` and `epoch_id` big-endian, then the
    /// score as a presence byte followed by the big-endian `u128` atomics of its `Decimal`, all
    /// zero when absent.
    ///
    /// This is a frozen wire format, shared by the contract and every verifier. It is exact, so
    /// a served score differing from the committed one by any amount yields another leaf, and
    /// it commits no per-kind medians, so adding a measurement kind leaves it unchanged.
    /// Changing it re-folds the whole accumulator under a new tag.
    fn digest_leaf(&self) -> Vec<u8> {
        let mut leaf = vec![0u8; Self::LEAF_LEN];
        leaf[0] = Self::LEAF_TAG;
        leaf[1..5].copy_from_slice(&self.node_id.to_be_bytes());
        leaf[5..9].copy_from_slice(&self.epoch_id.to_be_bytes());

        // an absent score leaves its presence byte and atomics zeroed
        if let Some(score) = self.score {
            leaf[9] = 1;
            leaf[10..].copy_from_slice(&score.value().atomics().u128().to_be_bytes());
        }
        leaf
    }
}

#[cw_serde]
pub struct LatestPerformancePagedResponse {
    pub performance: Vec<LatestPerformance>,
    pub start_next_after: Option<NodeId>,
}

/// The bundle a fallback resolved to, by epoch, with its medians.
#[cw_serde]
pub struct ResolvedMedians {
    pub epoch_id: EpochId,
    pub medians: KindMedians,
}

/// Everything rewarding used for a node in an epoch, or would have.
#[cw_serde]
pub struct RewardingInputsResponse {
    pub requested_epoch_id: EpochId,
    pub source: Option<ResolvedMedians>,
    pub weights: Option<EpochWeights>,
    pub score: Option<Percent>,
}

#[cw_serde]
pub struct RewardingScoreResponse {
    pub score: Option<Percent>,
}

#[cw_serde]
pub struct WeightsResponse {
    pub weights: Option<EpochWeights>,
}

#[cw_serde]
pub struct LastKnownEpochResponse {
    pub epoch_id: Option<EpochId>,
}

#[cw_serde]
pub struct NetworkMonitorInformation {
    pub details: NetworkMonitorDetails,
    pub current_submission_metadata: NetworkMonitorSubmissionMetadata,
}

#[cw_serde]
pub struct NetworkMonitorResponse {
    pub info: Option<NetworkMonitorInformation>,
}

#[cw_serde]
pub struct NetworkMonitorsPagedResponse {
    pub info: Vec<NetworkMonitorInformation>,
    pub start_next_after: Option<String>,
}

#[cw_serde]
pub struct RetiredNetworkMonitorsPagedResponse {
    pub info: Vec<RetiredNetworkMonitor>,
    pub start_next_after: Option<String>,
}

#[cw_serde]
#[derive(Default)]
pub struct BatchSubmissionResult {
    pub accepted_scores: u64,
    pub non_existent_nodes: Vec<NodeId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_json, to_json_string};

    fn p(raw: impl AsRef<str>) -> Percent {
        raw.as_ref().parse().unwrap()
    }

    fn ps(raw: &[&str]) -> Vec<Percent> {
        raw.iter().map(p).collect()
    }

    fn vals(results: &NodeResults) -> Vec<Percent> {
        results.values().collect()
    }

    fn medians(liveness: Option<&str>, stress: Option<&str>, config: Option<&str>) -> KindMedians {
        KindMedians {
            liveness: liveness.map(p),
            stress: stress.map(p),
            config: config.map(p),
        }
    }

    fn weights(liveness: &str, stress: &str) -> Weights {
        Weights {
            liveness: p(liveness),
            stress: p(stress),
        }
    }

    #[test]
    fn measurements_omit_absent_kinds_and_round_trip_on_chain() {
        let values = Measurements::default()
            .with_liveness(p("0.5"))
            .with_config(p("1"));
        let json = to_json_string(&values).unwrap();
        assert_eq!(json, r#"{"l":"0.5","c":"1"}"#);

        let back: Measurements = from_json(json.as_bytes()).unwrap();
        assert_eq!(back, values);
        assert_eq!(back.stress, None);

        let empty: Measurements = from_json(b"{}").unwrap();
        assert!(empty.is_empty());
        assert!(!values.is_empty());
    }

    #[test]
    fn kind_medians_use_full_field_names() {
        assert_eq!(
            to_json_string(&medians(Some("0.5"), None, Some("1"))).unwrap(),
            r#"{"liveness":"0.5","stress":null,"config":"1"}"#
        );
    }

    #[test]
    fn node_submission_uses_short_field_names() {
        let submission = NodeSubmission {
            node_id: 7,
            measurements: Measurements::default().with_liveness(p("0.95")),
        };
        assert_eq!(
            to_json_string(&submission).unwrap(),
            r#"{"n":7,"m":{"l":"0.95"}}"#
        );
    }

    #[test]
    fn node_results_are_stored_as_integer_percents() {
        let mut results = NodeResults::new(p("0.93"));
        results.insert_new(p("0.97"));
        results.insert_new(p("0.95"));

        let json = to_json_string(&results).unwrap();
        assert_eq!(json, "[93,95,97]");

        let back: NodeResults = from_json(json.as_bytes()).unwrap();
        assert_eq!(vals(&back), ps(&["0.93", "0.95", "0.97"]));
    }

    #[test]
    fn node_results_round_trip_the_bounds() {
        let mut results = NodeResults::new(p("1"));
        results.insert_new(p("0"));

        let json = to_json_string(&results).unwrap();
        assert_eq!(json, "[0,100]");

        let back: NodeResults = from_json(json.as_bytes()).unwrap();
        assert_eq!(vals(&back), ps(&["0", "1"]));
    }

    #[test]
    fn node_results_reject_invalid_stored_values() {
        assert!(from_json::<NodeResults>(b"[101]").is_err());
        assert!(from_json::<NodeResults>(b"[]").is_err());
    }

    #[test]
    fn node_results_insertion() {
        let initial = NodeResults::new(p("0.5"));

        let mut smaller = initial.clone();
        let mut greater = initial.clone();

        smaller.insert_new(p("0.4"));
        greater.insert_new(p("0.6"));

        assert_eq!(vals(&smaller), ps(&["0.4", "0.5"]));
        assert_eq!(vals(&greater), ps(&["0.5", "0.6"]));

        let mut another = NodeResults::new(p("0.1"));
        for raw in ["0.4", "0.5", "0.6", "0.6", "1.0"] {
            another.insert_new(p(raw));
        }
        another.insert_new(p("0.6"));
        another.insert_new(p("0.2"));
        another.insert_new(p("0.7"));
        another.insert_new(p("0.3"));
        another.insert_new(p("0.3"));
        another.insert_new(p("0.55"));

        assert_eq!(
            vals(&another),
            ps(&[
                "0.1", "0.2", "0.3", "0.3", "0.4", "0.5", "0.55", "0.6", "0.6", "0.6", "0.7", "1.0"
            ])
        );
    }

    #[test]
    fn values_are_rounded_half_up_on_insert() {
        let mut results = NodeResults::new(p("0.955"));
        results.insert_new(p("0.10"));
        results.insert_new(p("0.5"));
        assert_eq!(vals(&results), ps(&["0.1", "0.5", "0.96"]));
    }

    fn results(raw: &[&str]) -> NodeResults {
        let mut results = NodeResults::new(p(raw[0]));
        for raw in &raw[1..] {
            results.insert_new(p(raw));
        }
        results
    }

    #[test]
    fn node_results_median() {
        assert_eq!(results(&["0.1"]).median(), p("0.1"));
        assert_eq!(results(&["0.1", "0.2"]).median(), p("0.15"));
        assert_eq!(results(&["0.1", "0.2", "0.3"]).median(), p("0.2"));
        assert_eq!(results(&["0.1", "0.2", "0.3", "0.4"]).median(), p("0.25"));
        assert_eq!(
            results(&["0.1", "0.2", "0.3", "0.4", "0.5"]).median(),
            p("0.3")
        );
        assert_eq!(
            results(&["0", "0", "1", "1", "1", "1", "1"]).median(),
            p("1")
        );
    }

    fn two_monitor_bundle() -> EpochNodeMeasurements {
        let mut bundle = EpochNodeMeasurements::new(
            Measurements::default()
                .with_liveness(p("0.9"))
                .with_config(p("1")),
        );
        bundle.insert(
            Measurements::default()
                .with_liveness(p("0.8"))
                .with_stress(p("0.7"))
                .with_config(p("1")),
        );
        bundle
    }

    #[test]
    fn bundle_merges_kind_subsets_from_two_monitors() {
        let bundle = two_monitor_bundle();
        assert_eq!(vals(bundle.liveness.as_ref().unwrap()), ps(&["0.8", "0.9"]));
        assert_eq!(vals(bundle.stress.as_ref().unwrap()), ps(&["0.7"]));
        assert_eq!(vals(bundle.config.as_ref().unwrap()), ps(&["1", "1"]));
    }

    #[test]
    fn bundle_medians_are_per_kind() {
        assert_eq!(
            two_monitor_bundle().medians(),
            medians(Some("0.85"), Some("0.7"), Some("1"))
        );
    }

    #[test]
    fn bundle_json_shape_round_trips_on_chain() {
        let bundle = two_monitor_bundle();
        let json = to_json_string(&bundle).unwrap();
        assert_eq!(json, r#"{"l":[80,90],"s":[70],"c":[100,100]}"#);

        let back: EpochNodeMeasurements = from_json(json.as_bytes()).unwrap();
        assert_eq!(back, bundle);

        let partial: EpochNodeMeasurements = from_json(br#"{"c":[100]}"#).unwrap();
        assert_eq!(partial.liveness, None);
        assert_eq!(partial.medians(), medians(None, None, Some("1")));
    }

    #[test]
    fn weights_must_sum_to_exactly_one() {
        assert_eq!(
            weights("0.7", "0.2").validate(),
            Err(NymPerformanceContractError::WeightsDoNotSumToOne {
                total: Decimal::percent(90)
            })
        );
        assert_eq!(
            weights("0", "0").validate(),
            Err(NymPerformanceContractError::WeightsDoNotSumToOne {
                total: Decimal::zero()
            })
        );
        assert_eq!(weights("0.7", "0.3").validate(), Ok(()));
        assert_eq!(weights("1", "0").validate(), Ok(()));
    }

    #[test]
    fn a_missing_weight_field_is_zero() {
        assert_eq!(
            to_json_string(&weights("0.7", "0.3")).unwrap(),
            r#"{"liveness":"0.7","stress":"0.3"}"#
        );
        let back: Weights = from_json(br#"{"liveness":"1"}"#).unwrap();
        assert_eq!(back, weights("1", "0"));
    }

    #[test]
    fn score_of_a_single_applied_kind_is_its_median_times_config() {
        let m = medians(Some("0.8"), None, Some("0.5"));
        assert_eq!(weights("0.7", "0.3").score(m), Some(p("0.4")));
    }

    #[test]
    fn score_of_two_applied_kinds_uses_their_declared_shares() {
        let m = medians(Some("0.9"), Some("0.5"), Some("1"));
        assert_eq!(weights("0.7", "0.3").score(m), Some(p("0.78")));
    }

    #[test]
    fn config_gates_every_kind() {
        let m = medians(Some("1"), Some("1"), Some("0.5"));
        assert_eq!(weights("0.7", "0.3").score(m), Some(p("0.5")));
    }

    #[test]
    fn a_zero_weighted_kind_does_not_contribute() {
        let m = medians(Some("1"), Some("0"), Some("1"));
        assert_eq!(weights("1", "0").score(m), Some(p("1")));
    }

    #[test]
    fn no_applied_routing_kind_yields_no_score() {
        let w = weights("1", "0");
        assert_eq!(w.score(medians(None, None, Some("1"))), None);
        assert_eq!(w.score(medians(None, Some("0.9"), Some("1"))), None);
    }

    #[test]
    fn missing_config_yields_no_score() {
        assert_eq!(
            weights("1", "0").score(medians(Some("0.9"), None, None)),
            None
        );
    }

    /// If this fails, read `Weights::score`'s doc before changing the expected values.
    #[test]
    fn score_outputs_are_frozen() {
        let frozen = |stored: &str, weights: Weights, expected: Option<&str>| {
            let bundle: EpochNodeMeasurements = from_json(stored.as_bytes()).unwrap();
            assert_eq!(weights.score(bundle.medians()), expected.map(p), "{stored}");
        };

        // an absent kind is renormalised away, one monitor suffices, and config multiplies
        frozen(r#"{"l":[80],"c":[50]}"#, weights("0.7", "0.3"), Some("0.4"));
        // an even count averages its middle values, and 0.805 rounds half up to a whole percent
        frozen(
            r#"{"l":[80,90],"s":[70],"c":[100,100]}"#,
            weights("0.7", "0.3"),
            Some("0.81"),
        );
        // the averaged median is rounded (0.605 to 0.61) before scoring
        frozen(r#"{"s":[60,61],"c":[90]}"#, weights("0", "1"), Some("0.55"));
        // an odd count takes its middle value, with no outlier rejection
        frozen(
            r#"{"l":[50,95,99],"c":[100]}"#,
            weights("1", "0"),
            Some("0.95"),
        );
        // a zero-weighted kind contributes nothing
        frozen(
            r#"{"l":[100],"s":[0],"c":[100]}"#,
            weights("1", "0"),
            Some("1"),
        );
        // no weighted kind measured, or no config, yields no score
        frozen(r#"{"s":[90],"c":[100]}"#, weights("1", "0"), None);
        frozen(r#"{"l":[90]}"#, weights("1", "0"), None);
    }

    fn latest(node_id: NodeId, epoch_id: EpochId, score: Option<&str>) -> LatestPerformance {
        LatestPerformance {
            node_id,
            epoch_id,
            score: score.map(p),
        }
    }

    #[test]
    fn digest_leaf_layout_is_as_documented() {
        let header = [&[0x01][..], &[0, 0, 0, 7], &[0, 0, 0, 0x0a]].concat();
        let half_atomics = 500_000_000_000_000_000u128.to_be_bytes();

        assert_eq!(
            latest(7, 10, Some("0.5")).digest_leaf(),
            [&header[..], &[0x01], &half_atomics].concat()
        );
        assert_eq!(
            latest(7, 10, None).digest_leaf(),
            [&header[..], &[0x00], &[0; 16]].concat()
        );
    }

    #[test]
    fn digest_leaf_commits_every_field() {
        let leaf = latest(7, 10, Some("0.5")).digest_leaf();
        assert_ne!(latest(8, 10, Some("0.5")).digest_leaf(), leaf);
        assert_ne!(latest(7, 11, Some("0.5")).digest_leaf(), leaf);
        assert_ne!(latest(7, 10, Some("0.51")).digest_leaf(), leaf);
    }

    #[test]
    fn digest_leaf_tells_an_absent_score_from_a_present_zero() {
        assert_ne!(
            latest(7, 10, None).digest_leaf(),
            latest(7, 10, Some("0")).digest_leaf()
        );
    }

    #[test]
    fn digest_leaf_commits_the_score_exactly() {
        assert_ne!(
            latest(7, 10, Some("0.85")).digest_leaf(),
            latest(7, 10, Some("0.8549")).digest_leaf()
        );
    }
}
