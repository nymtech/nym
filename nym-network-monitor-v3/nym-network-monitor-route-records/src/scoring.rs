// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NodeId;
use crate::test_message::TestMessage;
use std::collections::{BTreeMap, HashMap};

/// Turns test messages into each node's five-hop liveness.
pub trait ScoringStrategy {
    /// Scores test messages given in send order, with each node's most recent one-hop score where it has one; a node it cannot score is absent.
    fn score(
        &self,
        test_messages: &[TestMessage],
        one_hop_scores: &HashMap<NodeId, f64>,
    ) -> BTreeMap<NodeId, f64>;
}

/// The performance monitor specification's rule: a lost message's blame goes to the nodes on it with the longest fail sequence.
#[derive(Debug, Clone, Copy)]
pub struct FailSequenceRule;

impl ScoringStrategy for FailSequenceRule {
    // the one-hop scores are unused until research specifies how they should change the blame
    fn score(
        &self,
        test_messages: &[TestMessage],
        _one_hop_scores: &HashMap<NodeId, f64>,
    ) -> BTreeMap<NodeId, f64> {
        FailSequenceCounts::over(test_messages).into_scores()
    }
}

/// What the fail-sequence rule keeps for every node while it scores.
#[derive(Default)]
struct FailSequenceCounts {
    nodes: BTreeMap<NodeId, NodeCounts>,
}

impl FailSequenceCounts {
    /// The counts after every test message, taken in send order.
    fn over(test_messages: &[TestMessage]) -> Self {
        let mut counts = FailSequenceCounts::default();
        for message in test_messages {
            if message.came_back {
                counts.count_came_back(message.path);
            } else {
                counts.count_lost(message.path);
            }
        }
        counts
    }

    /// Every node on the path gains a positive sample and its fail sequence ends.
    fn count_came_back(&mut self, path: [NodeId; 3]) {
        for node_id in path {
            let node = self.node(node_id);
            node.positive_samples += 1.0;
            node.fail_sequence = 0;
        }
    }

    /// Every node on the path extends its fail sequence, and those with the longest share one negative sample.
    fn count_lost(&mut self, path: [NodeId; 3]) {
        let fail_sequences = path.map(|node_id| {
            let node = self.node(node_id);
            node.fail_sequence += 1;
            node.fail_sequence
        });
        let longest = fail_sequences.into_iter().fold(0, u32::max);
        let suspects: Vec<NodeId> = path
            .into_iter()
            .zip(fail_sequences)
            .filter(|&(_, fail_sequence)| fail_sequence == longest)
            .map(|(node_id, _)| node_id)
            .collect();

        let share = 1.0 / suspects.len() as f64;
        for node_id in suspects {
            self.node(node_id).negative_samples += share;
        }
    }

    fn node(&mut self, node_id: NodeId) -> &mut NodeCounts {
        self.nodes.entry(node_id).or_default()
    }

    /// Each node's score; a node with no sample has none and is left out.
    fn into_scores(self) -> BTreeMap<NodeId, f64> {
        self.nodes
            .into_iter()
            .filter_map(|(node_id, node)| Some((node_id, node.score()?)))
            .collect()
    }
}

/// One node's running counts under the fail-sequence rule.
#[derive(Default)]
struct NodeCounts {
    positive_samples: f64,
    negative_samples: f64,
    fail_sequence: u32,
}

impl NodeCounts {
    /// Positive samples over all samples, or `None` without a sample.
    fn score(&self) -> Option<f64> {
        let samples = self.positive_samples + self.negative_samples;
        (samples > 0.0).then(|| self.positive_samples / samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_specifications_worked_example() {
        let scores = FailSequenceRule.score(&worked_example(), &HashMap::new());
        let counts = FailSequenceCounts::over(&worked_example());

        assert_score(&scores, 1, 6.0 / 7.0);
        assert_score(&scores, 2, 1.0);
        assert_score(&scores, 3, 1.0);
        assert_score(&scores, 4, 1.0);
        assert_score(&scores, 5, 0.3);
        assert_score(&scores, 6, 6.0 / 7.0);
        assert_close(counts.nodes[&5].positive_samples, 1.0);
        assert_close(counts.nodes[&5].negative_samples, 7.0 / 3.0);
    }

    #[test]
    fn a_first_failure_is_shared_by_everyone_on_the_path() {
        let messages = [message([1, 2, 3], true), message([1, 2, 3], false)];
        let counts = FailSequenceCounts::over(&messages);

        for node_id in [1, 2, 3] {
            assert_close(counts.nodes[&node_id].negative_samples, 1.0 / 3.0);
        }
    }

    #[test]
    fn a_repeat_offender_takes_the_whole_sample() {
        let messages = [message([1, 2, 3], false), message([1, 4, 5], false)];
        let counts = FailSequenceCounts::over(&messages);
        let scores = FailSequenceRule.score(&messages, &HashMap::new());

        assert_close(counts.nodes[&1].negative_samples, 1.0 / 3.0 + 1.0);
        for node_id in [4, 5] {
            assert_close(counts.nodes[&node_id].negative_samples, 0.0);
            assert!(
                !scores.contains_key(&node_id),
                "node {node_id} has no sample"
            );
        }
    }

    #[test]
    fn the_one_hop_scores_do_not_change_the_result() {
        let one_hop_scores = HashMap::from([(1, 1.0), (5, 0.0)]);
        assert_eq!(
            FailSequenceRule.score(&worked_example(), &HashMap::new()),
            FailSequenceRule.score(&worked_example(), &one_hop_scores),
        );
    }

    /// The specification's six-message example, with its gateways left out.
    fn worked_example() -> Vec<TestMessage> {
        vec![
            message([1, 2, 3], true),
            message([4, 5, 6], true),
            message([1, 5, 6], false),
            message([4, 5, 3], false),
            message([1, 2, 6], true),
            message([1, 5, 3], false),
        ]
    }

    fn message(path: [NodeId; 3], came_back: bool) -> TestMessage {
        TestMessage {
            sent_at_ms: 0,
            path,
            came_back,
        }
    }

    fn assert_score(scores: &BTreeMap<NodeId, f64>, node_id: NodeId, expected: f64) {
        assert_close(scores[&node_id], expected);
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }
}
