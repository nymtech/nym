// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::{
    fragmentation::fragment::Fragment,
    packet::{EncryptedLpPacket, LpFrame, frame::LpFrameKind},
};

use crate::packet::{SimDisplay, WirePacketFormat};

impl WirePacketFormat for EncryptedLpPacket {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(EncryptedLpPacket::decode(bytes)?)
    }
}

impl SimDisplay for EncryptedLpPacket {
    /// The envelope, which is the whole of what a node sees before it decrypts.
    ///
    /// The session tells it which key to try and the counter orders the stream; the rest is bytes
    /// it cannot read until it has used one against the other.
    fn describe(&self) -> String {
        let header = self.outer_header();
        format!(
            "session {:08x}  no. {:<4}  {} B sealed",
            header.receiver_idx,
            header.counter,
            self.ciphertext().len()
        )
    }
}

impl SimDisplay for LpFrame {
    /// What the envelope turned out to be holding.
    ///
    /// The kind is the LP framing proper: it says what the node is expected to do with the content,
    /// which is still a sphinx packet it has no key for.
    fn describe(&self) -> String {
        format!("{}  {} B", framing(self), self.content.len())
    }
}

/// A frame's kind and whatever that kind has to add, following it into any frame it carries.
///
/// Kept apart from [`SimDisplay::describe`] so a nested frame can be named without a size: only a
/// piece of it is present, and claiming the whole would be a lie.
fn framing(frame: &LpFrame) -> String {
    let kind = format!("{:?}", frame.kind());

    match frame.kind() {
        LpFrameKind::FragmentedData => {
            let Ok(fragment) = Fragment::try_from(frame.clone()) else {
                return kind;
            };

            let position = format!(
                "{kind} {}/{}",
                fragment.current_fragment() + 1,
                fragment.total_fragments()
            );

            if fragment.current_fragment() != 0 {
                return position;
            }

            // decoding takes a header and whatever follows it, so one fragment's worth is enough
            match LpFrame::decode(&frame.content) {
                Ok(inner) => format!("{position} → {}", framing(&inner)),
                Err(_) => position,
            }
        }

        _ => kind,
    }
}
