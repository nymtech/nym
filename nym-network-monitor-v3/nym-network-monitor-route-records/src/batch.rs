// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NodeId;
use anyhow::{Context, bail};

/// One test packet a session sent, and what came back of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// Unix time in milliseconds at which the packet was sent.
    pub sent_at_ms: u64,

    /// Node ids of the packet's mixnodes, in path order.
    pub packet_route: [NodeId; 3],

    /// Node ids of the SURB-ack's mixnodes, in path order.
    pub ack_route: [NodeId; 3],

    /// Whether the payload came back before the straggler wait ended.
    pub payload_back: bool,

    /// Whether the SURB-ack came back before the straggler wait ended.
    pub ack_back: bool,
}

impl Record {
    fn encode_into(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.sent_at_ms.to_be_bytes());
        for node_id in self.packet_route.iter().chain(&self.ack_route) {
            bytes.extend_from_slice(&node_id.to_be_bytes());
        }
        bytes.push(self.payload_back as u8);
        bytes.push(self.ack_back as u8);
    }

    fn decode(bytes: &mut &[u8]) -> anyhow::Result<Record> {
        Ok(Record {
            sent_at_ms: u64::from_be_bytes(take(bytes)?),
            packet_route: decode_route(bytes)?,
            ack_route: decode_route(bytes)?,
            payload_back: decode_bool(bytes)?,
            ack_back: decode_bool(bytes)?,
        })
    }
}

/// What a session states once for all of its records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchHeader {
    /// Node id of the gateway the session ran through.
    pub gateway_node_id: NodeId,

    /// Mixnet epoch of the active set the session used.
    pub mixnet_epoch: u32,

    /// Unix time in milliseconds at which the session started.
    pub session_start_ms: u64,
}

impl BatchHeader {
    fn encode_into(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.gateway_node_id.to_be_bytes());
        bytes.extend_from_slice(&self.mixnet_epoch.to_be_bytes());
        bytes.extend_from_slice(&self.session_start_ms.to_be_bytes());
    }

    fn decode(bytes: &mut &[u8]) -> anyhow::Result<BatchHeader> {
        Ok(BatchHeader {
            gateway_node_id: NodeId::from_be_bytes(take(bytes)?),
            mixnet_epoch: u32::from_be_bytes(take(bytes)?),
            session_start_ms: u64::from_be_bytes(take(bytes)?),
        })
    }
}

/// A session's records in the order sent, behind its header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub header: BatchHeader,
    pub records: Vec<Record>,
}

impl Batch {
    /// Packs the batch into its binary format.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.header.encode_into(&mut bytes);
        for record in &self.records {
            record.encode_into(&mut bytes);
        }
        bytes
    }

    /// Reads a batch, refusing anything that does not fit the format.
    pub fn decode(mut bytes: &[u8]) -> anyhow::Result<Batch> {
        let header = BatchHeader::decode(&mut bytes)?;
        let mut records = Vec::new();
        while !bytes.is_empty() {
            records.push(Record::decode(&mut bytes)?);
        }
        Ok(Batch { header, records })
    }
}

/// Splits the next `N` bytes off the front of `bytes`.
fn take<const N: usize>(bytes: &mut &[u8]) -> anyhow::Result<[u8; N]> {
    let (head, rest) = bytes
        .split_first_chunk::<N>()
        .context("the batch ends early")?;
    *bytes = rest;
    Ok(*head)
}

fn decode_route(bytes: &mut &[u8]) -> anyhow::Result<[NodeId; 3]> {
    Ok([
        NodeId::from_be_bytes(take(bytes)?),
        NodeId::from_be_bytes(take(bytes)?),
        NodeId::from_be_bytes(take(bytes)?),
    ])
}

fn decode_bool(bytes: &mut &[u8]) -> anyhow::Result<bool> {
    match take::<1>(bytes)? {
        [0] => Ok(false),
        [1] => Ok(true),
        [other] => bail!("{other} is not a flag"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_reads_back_as_written() {
        let batch = test_batch();
        assert_eq!(Batch::decode(&batch.encode()).unwrap(), batch);
    }

    #[test]
    fn a_batch_ending_partway_through_a_record_is_refused() {
        let mut bytes = test_batch().encode();
        bytes.pop();
        assert!(Batch::decode(&bytes).is_err());
    }

    #[test]
    fn a_flag_byte_other_than_0_or_1_is_refused() {
        let mut bytes = test_batch().encode();
        *bytes.last_mut().unwrap() = 2;
        assert!(Batch::decode(&bytes).is_err());
    }

    fn test_batch() -> Batch {
        let start = 1_764_000_000_000;
        let record = |offset_ms, payload_back, ack_back| Record {
            sent_at_ms: start + offset_ms,
            packet_route: [11, 100_021, 3_000_031],
            ack_route: [12, 100_022, 3_000_032],
            payload_back,
            ack_back,
        };
        Batch {
            header: BatchHeader {
                gateway_node_id: 70_000,
                mixnet_epoch: 31_337,
                session_start_ms: start,
            },
            records: vec![
                record(0, true, true),
                record(18, true, false),
                record(18, false, true),
                record(65, false, false),
            ],
        }
    }
}
