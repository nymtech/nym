// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NodeId;
use crate::batch::Batch;

/// A session that lost more than this share of its test messages is discarded.
const MAX_LOSS_RATE: f64 = 0.5;

/// One packet's or SURB-ack's trip through three mixnodes, as the scoring sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestMessage {
    /// Unix time in milliseconds at which its test packet was sent; an ack's is that of the packet that carried it.
    pub sent_at_ms: u64,

    /// Node ids of the three mixnodes on its path, in path order.
    pub path: [NodeId; 3],

    /// Whether it came back.
    pub came_back: bool,
}

/// A session's test messages in the order sent, with the gateway it ran through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTestMessages {
    pub gateway_node_id: NodeId,
    pub test_messages: Vec<TestMessage>,
}

impl SessionTestMessages {
    /// Turns a session's records into test messages under the two ack rules.
    pub fn from_batch(batch: &Batch) -> SessionTestMessages {
        let mut test_messages = Vec::with_capacity(2 * batch.records.len());
        for record in &batch.records {
            // a returned ack shows the packet's mixnodes delivered it, even if the payload was lost
            let reached_gateway = record.payload_back || record.ack_back;
            test_messages.push(TestMessage {
                sent_at_ms: record.sent_at_ms,
                path: record.packet_route,
                came_back: reached_gateway,
            });

            // an ack is launched only once its packet reached the gateway
            if reached_gateway {
                test_messages.push(TestMessage {
                    sent_at_ms: record.sent_at_ms,
                    path: record.ack_route,
                    came_back: record.ack_back,
                });
            }
        }

        SessionTestMessages {
            gateway_node_id: batch.header.gateway_node_id,
            test_messages,
        }
    }

    /// Whether the session is discarded before scoring, judged against the sessions of its run.
    pub fn is_suspected(&self, run: &[SessionTestMessages], margin: f64) -> bool {
        let Some(loss_rate) = self.loss_rate() else {
            return false;
        };
        let best_loss_rate = run
            .iter()
            .filter_map(SessionTestMessages::loss_rate)
            .fold(loss_rate, f64::min);

        loss_rate > MAX_LOSS_RATE || loss_rate > best_loss_rate + margin
    }

    fn loss_rate(&self) -> Option<f64> {
        if self.test_messages.is_empty() {
            return None;
        }
        let lost = self.test_messages.iter().filter(|m| !m.came_back).count();
        Some(lost as f64 / self.test_messages.len() as f64)
    }
}

/// Merges the kept sessions' test messages into send order, equal times ordered by gateway node id.
pub fn merge_in_send_order(mut sessions: Vec<SessionTestMessages>) -> Vec<TestMessage> {
    // both sorts are stable, so equal times keep the gateway order and each session's own order
    sessions.sort_by_key(|session| session.gateway_node_id);
    let mut merged: Vec<TestMessage> = sessions
        .into_iter()
        .flat_map(|session| session.test_messages)
        .collect();
    merged.sort_by_key(|message| message.sent_at_ms);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::{BatchHeader, Record};

    const PACKET_ROUTE: [NodeId; 3] = [1, 2, 3];
    const ACK_ROUTE: [NodeId; 3] = [4, 5, 6];
    const SENT_AT_MS: u64 = 1_764_000_000_000;
    const MARGIN: f64 = 0.05;

    #[test]
    fn both_back_yields_two_successes() {
        assert_eq!(
            record_test_messages(true, true),
            vec![message(PACKET_ROUTE, true), message(ACK_ROUTE, true)]
        );
    }

    #[test]
    fn only_the_payload_back_fails_the_ack() {
        assert_eq!(
            record_test_messages(true, false),
            vec![message(PACKET_ROUTE, true), message(ACK_ROUTE, false)]
        );
    }

    #[test]
    fn only_the_ack_back_counts_the_packet_as_delivered() {
        assert_eq!(
            record_test_messages(false, true),
            vec![message(PACKET_ROUTE, true), message(ACK_ROUTE, true)]
        );
    }

    #[test]
    fn neither_back_fails_the_packet_and_yields_nothing_for_the_ack() {
        assert_eq!(
            record_test_messages(false, false),
            vec![message(PACKET_ROUTE, false)]
        );
    }

    #[test]
    fn a_session_that_lost_most_of_its_messages_is_discarded() {
        // the other session is within the margin, so only the first test can discard it
        let run = [session_losing(1, 60), session_losing(2, 58)];
        assert!(run[0].is_suspected(&run, MARGIN));
    }

    #[test]
    fn the_worse_of_two_unequal_sessions_is_discarded() {
        let run = [session_losing(1, 2), session_losing(2, 9)];
        assert!(!run[0].is_suspected(&run, MARGIN));
        assert!(run[1].is_suspected(&run, MARGIN));
    }

    #[test]
    fn two_sessions_that_agree_are_both_kept() {
        let run = [session_losing(1, 8), session_losing(2, 10)];
        assert!(!run[0].is_suspected(&run, MARGIN));
        assert!(!run[1].is_suspected(&run, MARGIN));
    }

    #[test]
    fn a_lone_session_is_judged_on_its_own() {
        let run = [session_losing(1, 12)];
        assert!(!run[0].is_suspected(&run, MARGIN));
    }

    #[test]
    fn the_merge_orders_by_send_time_then_gateway_keeping_each_sessions_order() {
        let at = |sent_at_ms, first_node| TestMessage {
            sent_at_ms,
            path: [first_node, 0, 0],
            came_back: true,
        };
        let gateway_2 = SessionTestMessages {
            gateway_node_id: 2,
            test_messages: vec![at(10, 21), at(20, 22)],
        };
        let gateway_1 = SessionTestMessages {
            gateway_node_id: 1,
            test_messages: vec![at(10, 11), at(10, 12), at(30, 13)],
        };

        assert_eq!(
            merge_in_send_order(vec![gateway_2, gateway_1]),
            vec![at(10, 11), at(10, 12), at(10, 21), at(20, 22), at(30, 13)]
        );
    }

    fn record_test_messages(payload_back: bool, ack_back: bool) -> Vec<TestMessage> {
        let batch = Batch {
            header: BatchHeader {
                gateway_node_id: 100,
                mixnet_epoch: 1,
                session_start_ms: SENT_AT_MS,
            },
            records: vec![Record {
                sent_at_ms: SENT_AT_MS,
                packet_route: PACKET_ROUTE,
                ack_route: ACK_ROUTE,
                payload_back,
                ack_back,
            }],
        };
        SessionTestMessages::from_batch(&batch).test_messages
    }

    fn message(path: [NodeId; 3], came_back: bool) -> TestMessage {
        TestMessage {
            sent_at_ms: SENT_AT_MS,
            path,
            came_back,
        }
    }

    /// A session of 100 test messages, `lost_percent` of which were lost.
    fn session_losing(gateway_node_id: NodeId, lost_percent: usize) -> SessionTestMessages {
        let test_messages = (0..100)
            .map(|i| message(PACKET_ROUTE, i >= lost_percent))
            .collect();
        SessionTestMessages {
            gateway_node_id,
            test_messages,
        }
    }
}
