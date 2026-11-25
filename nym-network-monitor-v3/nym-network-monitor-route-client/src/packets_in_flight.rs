// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_monitor_route_records::NodeId;
use nym_network_monitor_route_records::batch::Record;
use nym_sphinx::params::SerializedFragmentIdentifier;
use time::OffsetDateTime;

/// The session's table of packets in flight: one record per packet, indexed by its sequence number.
#[derive(Debug, Default)]
pub(crate) struct PacketsInFlight {
    records: Vec<Record>,
}

impl PacketsInFlight {
    /// Records a packet about to be sent and returns the identifier it carries in its content and SURB-ack.
    pub(crate) fn insert(
        &mut self,
        packet_route: [NodeId; 3],
        ack_route: [NodeId; 3],
    ) -> SerializedFragmentIdentifier {
        let sequence_number = self.records.len() as u32;
        self.records.push(Record {
            sent_at_ms: unix_time_ms(),
            packet_route,
            ack_route,
            payload_back: false,
            ack_back: false,
        });

        // the sequence number big-endian, then a 0 byte
        let [a, b, c, d] = sequence_number.to_be_bytes();
        [a, b, c, d, 0]
    }
}

fn unix_time_ms() -> u64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as u64
}
