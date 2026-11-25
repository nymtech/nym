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

    /// Payloads and SURB-acks sent and not back yet.
    pending: usize,
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
        // its payload and its SURB-ack
        self.pending += 2;

        // the sequence number big-endian, then a 0 byte
        let [a, b, c, d] = sequence_number.to_be_bytes();
        [a, b, c, d, 0]
    }

    /// Marks as back the payload whose decrypted content starts with its packet's identifier.
    pub(crate) fn payload_back(&mut self, content: &[u8]) {
        if let Some(index) = self.index_named(content)
            && !self.records[index].payload_back
        {
            self.records[index].payload_back = true;
            self.pending -= 1;
        }
    }

    /// Marks as back the SURB-ack carrying this identifier.
    pub(crate) fn ack_back(&mut self, identifier: &[u8]) {
        if let Some(index) = self.index_named(identifier)
            && !self.records[index].ack_back
        {
            self.records[index].ack_back = true;
            self.pending -= 1;
        }
    }

    /// Whether every packet sent has had both its payload and its SURB-ack back.
    pub(crate) fn all_back(&self) -> bool {
        self.pending == 0
    }

    /// Closes the table: every record in the order sent, as it stands.
    pub(crate) fn into_records(self) -> Vec<Record> {
        self.records
    }

    /// The index of the packet an identifier names, if the session sent one under it.
    fn index_named(&self, identifier: &[u8]) -> Option<usize> {
        // the sequence number is the identifier's first 4 bytes
        let sequence_number = u32::from_be_bytes(*identifier.first_chunk::<4>()?) as usize;
        (sequence_number < self.records.len()).then_some(sequence_number)
    }
}

fn unix_time_ms() -> u64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTE: [NodeId; 3] = [11, 22, 33];

    #[test]
    fn a_returned_payload_and_ack_mark_only_their_own_record() {
        let mut table = PacketsInFlight::default();
        let first = table.insert(ROUTE, ROUTE);
        table.insert(ROUTE, ROUTE);

        table.payload_back(&decrypted_content(first));
        table.ack_back(&first);

        let records = table.into_records();
        assert!(records[0].payload_back && records[0].ack_back);
        assert!(!records[1].payload_back && !records[1].ack_back);
    }

    #[test]
    fn a_repeated_arrival_counts_once() {
        let mut table = PacketsInFlight::default();
        let first = table.insert(ROUTE, ROUTE);
        let second = table.insert(ROUTE, ROUTE);

        table.payload_back(&decrypted_content(first));
        table.payload_back(&decrypted_content(first));
        table.ack_back(&first);
        table.ack_back(&first);
        assert!(!table.all_back());

        table.payload_back(&decrypted_content(second));
        table.ack_back(&second);
        assert!(table.all_back());
    }

    #[test]
    fn an_identifier_naming_no_packet_is_ignored() {
        let mut table = PacketsInFlight::default();
        table.insert(ROUTE, ROUTE);

        // sequence number 1, never sent, and a content too short to carry one
        table.payload_back(&[0, 0, 0, 1, 0]);
        table.ack_back(&[0, 0, 0, 1, 0]);
        table.payload_back(&[0, 0]);
        table.ack_back(&[0, 0]);

        assert!(!table.all_back());
        let records = table.into_records();
        assert!(!records[0].payload_back && !records[0].ack_back);
    }

    #[test]
    fn all_back_needs_the_payload_and_the_ack() {
        let mut table = PacketsInFlight::default();
        let identifier = table.insert(ROUTE, ROUTE);

        table.payload_back(&decrypted_content(identifier));
        assert!(!table.all_back());

        table.ack_back(&identifier);
        assert!(table.all_back());
    }

    /// A returning payload's decrypted content: the identifier, padded as the loop packet builder pads it.
    fn decrypted_content(identifier: SerializedFragmentIdentifier) -> Vec<u8> {
        identifier.into_iter().chain([1]).chain([0; 32]).collect()
    }
}
