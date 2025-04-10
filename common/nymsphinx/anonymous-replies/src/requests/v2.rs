// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::requests::InvalidReplyRequestError;
use crate::ReplySurb;
use nym_sphinx_types::constants::PAYLOAD_KEY_SEED_SIZE;
use std::fmt::Display;
use std::iter::once;
use tracing::{error, warn};

const fn v2_reply_surb_serialised_len(num_hops: u8) -> usize {
    ReplySurb::BASE_OVERHEAD + num_hops as usize * PAYLOAD_KEY_SEED_SIZE
}

// sphinx doesn't support more than 5 hops (so cast to u8 is safe)
// ASSUMPTION: all surbs are generated with the same parameters (if they're not, then the client is hurting itself)
fn reply_surbs_hops(reply_surbs: &[ReplySurb]) -> u8 {
    reply_surbs
        .first()
        .map(|reply_surb| reply_surb.surb.materials_count() as u8)
        .unwrap_or_default()
}

fn v2_reply_surbs_serialised_len(surbs: &[ReplySurb]) -> usize {
    let num_surbs = surbs.len();
    let num_hops = reply_surbs_hops(surbs);

    // sanity checks; this should probably be removed later on
    if let Some(reply_surb) = surbs.first() {
        if !reply_surb.surb.uses_key_seeds() {
            error!("using v2 surbs encoding with legacy structure - the surbs will be unusable")
        }
    }

    // when serialising surbs are always prepended with u16-encoded count an u8-encoded number of hops
    3 + num_surbs * v2_reply_surb_serialised_len(num_hops)
}

// NUM_SURBS (u16) || HOPS (u8) || SURB_DATA
fn recover_reply_surbs_v2(
    bytes: &[u8],
) -> Result<(Vec<ReplySurb>, usize), InvalidReplyRequestError> {
    if bytes.len() < 2 {
        return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
    }

    // we're not attaching more than 65k surbs...
    let num_surbs = u16::from_be_bytes([bytes[0], bytes[1]]);
    let num_hops = bytes[2];
    let mut consumed = 3;

    let surb_size = ReplySurb::v2_serialised_len(num_hops);
    if bytes[consumed..].len() < num_surbs as usize * surb_size {
        return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
    }

    let mut reply_surbs = Vec::with_capacity(num_surbs as usize);
    for _ in 0..num_surbs as usize {
        let surb_bytes = &bytes[consumed..consumed + surb_size];
        let reply_surb = ReplySurb::from_bytes(surb_bytes)?;
        reply_surbs.push(reply_surb);

        consumed += surb_size;
    }

    Ok((reply_surbs, consumed))
}

fn reply_surbs_bytes_v2(reply_surbs: &[ReplySurb]) -> impl Iterator<Item = u8> + use<'_> {
    let num_surbs = reply_surbs.len() as u16;
    let num_hops = reply_surbs_hops(reply_surbs);

    num_surbs
        .to_be_bytes()
        .into_iter()
        .chain(once(num_hops))
        .chain(reply_surbs.iter().flat_map(|surb| surb.to_bytes()))
}

#[derive(Debug)]
pub struct DataV2 {
    pub message: Vec<u8>,
    pub reply_surbs: Vec<ReplySurb>,
}

impl Display for DataV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "V2 repliable {:.2} kiB data message with {} reply surbs attached",
            self.message.len() as f64 / 1024.0,
            self.reply_surbs.len(),
        )
    }
}

#[derive(Debug)]
pub struct AdditionalSurbsV2 {
    pub reply_surbs: Vec<ReplySurb>,
}

impl Display for AdditionalSurbsV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "V2 repliable additional surbs message ({} reply surbs attached)",
            self.reply_surbs.len(),
        )
    }
}

#[derive(Debug)]
pub struct HeartbeatV2 {
    pub additional_reply_surbs: Vec<ReplySurb>,
}

impl Display for HeartbeatV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "V2 repliable heartbeat message ({} reply surbs attached)",
            self.additional_reply_surbs.len(),
        )
    }
}

impl DataV2 {
    pub fn into_bytes(self) -> Vec<u8> {
        reply_surbs_bytes_v2(&self.reply_surbs)
            .chain(self.message)
            .collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        let (reply_surbs, n) = recover_reply_surbs_v2(bytes)?;
        Ok(DataV2 {
            message: bytes[n..].to_vec(),
            reply_surbs,
        })
    }

    pub fn serialized_len(&self) -> usize {
        v2_reply_surbs_serialised_len(&self.reply_surbs) + self.message.len()
    }
}

impl AdditionalSurbsV2 {
    pub fn into_bytes(self) -> Vec<u8> {
        reply_surbs_bytes_v2(&self.reply_surbs).collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        let (reply_surbs, n) = recover_reply_surbs_v2(bytes)?;
        if n != bytes.len() {
            let trailing = bytes.len() - n;
            warn!("trailing {trailing} bytes after v2 additional surbs message");
        }

        Ok(AdditionalSurbsV2 { reply_surbs })
    }

    pub fn serialized_len(&self) -> usize {
        v2_reply_surbs_serialised_len(&self.reply_surbs)
    }
}

impl HeartbeatV2 {
    pub fn into_bytes(self) -> Vec<u8> {
        reply_surbs_bytes_v2(&self.additional_reply_surbs).collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        let (additional_reply_surbs, n) = recover_reply_surbs_v2(bytes)?;
        if n != bytes.len() {
            let trailing = bytes.len() - n;
            warn!("trailing {trailing} bytes after v2 heartbeat message");
        }

        Ok(HeartbeatV2 {
            additional_reply_surbs,
        })
    }

    pub fn serialized_len(&self) -> usize {
        v2_reply_surbs_serialised_len(&self.additional_reply_surbs)
    }
}
