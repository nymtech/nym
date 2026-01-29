use std::collections::HashMap;

use tracing::warn;

use crate::http::models::{
    Gateway,
    gw_probe::{LastProbeResult, ScoreValue},
};

pub(crate) fn calculate_socks5_percentiles(gateways: &[Gateway]) -> HashMap<String, ScoreValue> {
    let parsed_gateways = gateways
        .iter()
        // discard untested gateways
        .filter_map(|gw| {
            gw.last_probe_result
                .as_ref()
                .map(|res| (gw.gateway_identity_key.clone(), res))
        })
        // discard unparsable probe results (error)
        .filter_map(|(id, value)| {
            LastProbeResult::deserialize_with_fallback(value.to_owned())
                .inspect_err(|err| warn!("Failed to deserialize probe result: {err}"))
                .ok()
                .map(|parsed| (id, parsed))
        })
        .map(|(id, res)| {
            let latency = res
                .outcome
                .socks5
                // if socks5 is null, test failed or gw doesn't support it
                .and_then(|socks5| socks5.https_connectivity.https_latency_ms)
                .unwrap_or(0);

            (id, latency)
        })
        .collect::<Vec<(_, _)>>();

    score_from_sorted_latencies(parsed_gateways)
}

/// Assigns a score to each gateway based on their relative latency compared to
/// the whole set. Higher score = lower latency.
///
/// - latency == 0 => Offline
/// - nonzero buckets:
///   - High   = lowest 50%
///   - Medium = next 25%
///   - Low    = worst 25%
pub fn score_from_sorted_latencies(gateways: Vec<(String, u64)>) -> HashMap<String, ScoreValue> {
    // sort ascending
    let mut gateways = gateways;
    gateways.sort_by_cached_key(|(_, latency)| *latency);

    // as soon as you find the first nonzero latency, it's a boundary where non-zero starts
    let (offline_gws, online_gws): (Vec<_>, Vec<_>) =
        gateways.into_iter().partition(|(_, latency)| *latency == 0);

    let nonzero_count = online_gws.len();

    // x / 2 = 0.5x
    let high_end_idx = nonzero_count.div_ceil(2);
    // 3x / 4 = 0.75x
    let medium_end_idx = nonzero_count.saturating_mul(3).div_ceil(4);
    // `Low` gets assigned to everything else

    let mut result = HashMap::new();

    // first flag all the zero-latency as Offline
    for (id, _lat) in offline_gws {
        result.entry(id).or_insert(ScoreValue::Offline);
    }

    // secondly go over remaining non-zero elements, assign by rank within non-zero set
    for (idx, (id, _)) in online_gws.into_iter().enumerate() {
        let score = if idx < high_end_idx {
            ScoreValue::High
        } else if idx < medium_end_idx {
            ScoreValue::Medium
        } else {
            ScoreValue::Low
        };

        result.entry(id).or_insert(score);
    }

    result
}

#[cfg(test)]
mod socks5_score_calc_tests {
    // clippy complains despite imports being used
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn empty_input() {
        let result = score_from_sorted_latencies(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn all_offline() {
        let items = vec![
            ("a".to_string(), 0),
            ("b".to_string(), 0),
            ("c".to_string(), 0),
        ];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("a"), Some(&ScoreValue::Offline));
        assert_eq!(result.get("b"), Some(&ScoreValue::Offline));
        assert_eq!(result.get("c"), Some(&ScoreValue::Offline));
    }

    #[test]
    fn single_zero() {
        let items = vec![("a".to_string(), 0)];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("a"), Some(&ScoreValue::Offline));
    }

    #[test]
    fn single_nonzero() {
        // Single non-zero element: lowest 50% of 1 = ceil(1/2) = 1, so it's High
        let items = vec![("a".to_string(), 100)];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("a"), Some(&ScoreValue::High));
    }

    #[test]
    fn two_nonzero_elements() {
        // 2 non-zero: high_end = ceil(2/2) = 1, medium_end = ceil(6/4) = 2
        // idx 0 -> High, idx 1 -> Medium
        let items = vec![("a".to_string(), 100), ("b".to_string(), 200)];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("a"), Some(&ScoreValue::High));
        assert_eq!(result.get("b"), Some(&ScoreValue::Medium));
    }

    #[test]
    fn mix_zeros_and_nonzeros() {
        // Zeros become Offline, non-zeros get percentile scores
        let items = vec![
            ("offline1".to_string(), 0),
            ("fast".to_string(), 50),
            ("offline2".to_string(), 0),
            ("slow".to_string(), 200),
        ];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 4);
        assert_eq!(result.get("offline1"), Some(&ScoreValue::Offline));
        assert_eq!(result.get("offline2"), Some(&ScoreValue::Offline));
        // 2 non-zero: high_end = 1, medium_end = 2
        assert_eq!(result.get("fast"), Some(&ScoreValue::High));
        assert_eq!(result.get("slow"), Some(&ScoreValue::Medium));
    }

    #[test]
    fn unsorted_input() {
        // Input is unsorted, function should work regardless
        let items = vec![
            ("slow".to_string(), 300),
            ("fast".to_string(), 100),
            ("medium".to_string(), 200),
        ];
        let result = score_from_sorted_latencies(items);

        // 3 non-zero: high_end = ceil(3/2) = 2, medium_end = ceil(9/4) = 3
        // sorted: fast(100), medium(200), slow(300)
        // idx 0,1 -> High, idx 2 -> Medium
        assert_eq!(result.get("fast"), Some(&ScoreValue::High));
        assert_eq!(result.get("medium"), Some(&ScoreValue::High));
        assert_eq!(result.get("slow"), Some(&ScoreValue::Medium));
    }

    #[test]
    fn duplicate_ids_keeps_first() {
        // Duplicate IDs: first occurrence is kept
        let items = vec![
            ("dup".to_string(), 100), // first occurrence, fast
            ("other".to_string(), 200),
            ("dup".to_string(), 300), // duplicate, slow - should be ignored
        ];
        let result = score_from_sorted_latencies(items);

        // 3 non-zero: high_end = 2, medium_end = 3
        // sorted: dup(100), other(200), dup(300)
        // idx 0 -> High (dup first), idx 1 -> High (other), idx 2 -> Medium (dup second, ignored)
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("dup"), Some(&ScoreValue::High));
        assert_eq!(result.get("other"), Some(&ScoreValue::High));
    }

    #[test]
    fn four_nonzero_clean_percentiles() {
        // 4 non-zero: high_end = ceil(4/2) = 2, medium_end = ceil(12/4) = 3
        // idx 0,1 -> High (50%), idx 2 -> Medium (25%), idx 3 -> Low (25%)
        let items = vec![
            ("a".to_string(), 100),
            ("b".to_string(), 200),
            ("c".to_string(), 300),
            ("d".to_string(), 400),
        ];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 4);
        assert_eq!(result.get("a"), Some(&ScoreValue::High));
        assert_eq!(result.get("b"), Some(&ScoreValue::High));
        assert_eq!(result.get("c"), Some(&ScoreValue::Medium));
        assert_eq!(result.get("d"), Some(&ScoreValue::Low));
    }

    #[test]
    fn eight_nonzero_clean_percentiles() {
        // 8 non-zero: high_end = ceil(8/2) = 4, medium_end = ceil(24/4) = 6
        // idx 0-3 -> High, idx 4-5 -> Medium, idx 6-7 -> Low
        let items: Vec<(String, u64)> = (1..=8)
            .map(|i| (format!("node{}", i), i as u64 * 100))
            .collect();
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.len(), 8);
        assert_eq!(result.get("node1"), Some(&ScoreValue::High));
        assert_eq!(result.get("node2"), Some(&ScoreValue::High));
        assert_eq!(result.get("node3"), Some(&ScoreValue::High));
        assert_eq!(result.get("node4"), Some(&ScoreValue::High));
        assert_eq!(result.get("node5"), Some(&ScoreValue::Medium));
        assert_eq!(result.get("node6"), Some(&ScoreValue::Medium));
        assert_eq!(result.get("node7"), Some(&ScoreValue::Low));
        assert_eq!(result.get("node8"), Some(&ScoreValue::Low));
    }

    #[test]
    fn five_nonzero_ceiling_division() {
        // 5 non-zero: high_end = ceil(5/2) = 3, medium_end = ceil(15/4) = 4
        // idx 0,1,2 -> High (3), idx 3 -> Medium (1), idx 4 -> Low (1)
        let items: Vec<(String, u64)> = (1..=5)
            .map(|i| (format!("n{}", i), i as u64 * 10))
            .collect();
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.get("n1"), Some(&ScoreValue::High));
        assert_eq!(result.get("n2"), Some(&ScoreValue::High));
        assert_eq!(result.get("n3"), Some(&ScoreValue::High));
        assert_eq!(result.get("n4"), Some(&ScoreValue::Medium));
        assert_eq!(result.get("n5"), Some(&ScoreValue::Low));
    }

    #[test]
    fn seven_nonzero_ceiling_division() {
        // 7 non-zero: high_end = ceil(7/2) = 4, medium_end = ceil(21/4) = 6
        // idx 0-3 -> High (4), idx 4-5 -> Medium (2), idx 6 -> Low (1)
        let items: Vec<(String, u64)> = (1..=7)
            .map(|i| (format!("n{}", i), i as u64 * 10))
            .collect();
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.get("n1"), Some(&ScoreValue::High));
        assert_eq!(result.get("n2"), Some(&ScoreValue::High));
        assert_eq!(result.get("n3"), Some(&ScoreValue::High));
        assert_eq!(result.get("n4"), Some(&ScoreValue::High));
        assert_eq!(result.get("n5"), Some(&ScoreValue::Medium));
        assert_eq!(result.get("n6"), Some(&ScoreValue::Medium));
        assert_eq!(result.get("n7"), Some(&ScoreValue::Low));
    }

    #[test]
    fn three_nonzero() {
        // 3 non-zero: high_end = ceil(3/2) = 2, medium_end = ceil(9/4) = 3
        // idx 0,1 -> High, idx 2 -> Medium (no Low bucket)
        let items = vec![
            ("a".to_string(), 100),
            ("b".to_string(), 200),
            ("c".to_string(), 300),
        ];
        let result = score_from_sorted_latencies(items);

        assert_eq!(result.get("a"), Some(&ScoreValue::High));
        assert_eq!(result.get("b"), Some(&ScoreValue::High));
        assert_eq!(result.get("c"), Some(&ScoreValue::Medium));
    }
}
