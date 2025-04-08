// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::requests::InvalidReplyRequestError;
use crate::ReplySurb;
use log::warn;
use nym_sphinx_types::PAYLOAD_KEY_SIZE;
use std::fmt::Display;
use std::mem;

const fn v1_reply_surb_serialised_len() -> usize {
    // the SURB itself consists of SURB_header, first hop address and set of payload keys
    // for each hop (3x mix + egress)
    ReplySurb::BASE_OVERHEAD + 4 * PAYLOAD_KEY_SIZE
}

const fn v1_reply_surbs_serialised_len(surbs: &[ReplySurb]) -> usize {
    // when serialising surbs are always prepended with u32-encoded count
    4 + surbs.len() * v1_reply_surb_serialised_len()
}

// this recovery code is shared between all legacy variants containing reply surbs
// NUM_SURBS (u32) || SURB_DATA
fn recover_reply_surbs_v1(
    bytes: &[u8],
) -> Result<(Vec<ReplySurb>, usize), InvalidReplyRequestError> {
    let mut consumed = mem::size_of::<u32>();
    if bytes.len() < consumed {
        return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
    }
    let num_surbs = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

    let surb_size = v1_reply_surb_serialised_len();
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

// length (u32) prefixed reply surbs with legacy serialisation of 4 hops and full payload keys attached
fn reply_surbs_bytes_v1(reply_surbs: &[ReplySurb]) -> impl Iterator<Item = u8> + use<'_> {
    let num_surbs = reply_surbs.len() as u32;

    num_surbs
        .to_be_bytes()
        .into_iter()
        .chain(reply_surbs.iter().flat_map(|s| s.to_bytes()))
}

#[derive(Debug)]
pub struct DataV1 {
    pub message: Vec<u8>,
    pub reply_surbs: Vec<ReplySurb>,
}

impl Display for DataV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "V1 repliable {:.2} kiB data message with {} reply surbs attached",
            self.message.len() as f64 / 1024.0,
            self.reply_surbs.len(),
        )
    }
}

#[derive(Debug)]
pub struct AdditionalSurbsV1 {
    pub reply_surbs: Vec<ReplySurb>,
}

impl Display for AdditionalSurbsV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "V1 repliable additional surbs message ({} reply surbs attached)",
            self.reply_surbs.len(),
        )
    }
}

#[derive(Debug)]
pub struct HeartbeatV1 {
    pub additional_reply_surbs: Vec<ReplySurb>,
}

impl Display for HeartbeatV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "V1 repliable heartbeat message ({} reply surbs attached)",
            self.additional_reply_surbs.len(),
        )
    }
}

impl DataV1 {
    pub fn into_bytes(self) -> Vec<u8> {
        reply_surbs_bytes_v1(&self.reply_surbs)
            .chain(self.message)
            .collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        let (reply_surbs, n) = recover_reply_surbs_v1(bytes)?;
        Ok(DataV1 {
            message: bytes[n..].to_vec(),
            reply_surbs,
        })
    }

    pub fn serialized_len(&self) -> usize {
        v1_reply_surbs_serialised_len(&self.reply_surbs) + self.message.len()
    }
}

impl AdditionalSurbsV1 {
    pub fn into_bytes(self) -> Vec<u8> {
        reply_surbs_bytes_v1(&self.reply_surbs).collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        let (reply_surbs, n) = recover_reply_surbs_v1(bytes)?;
        if n != 0 {
            warn!("trailing {n} bytes after v1 additional surbs message");
        }

        Ok(AdditionalSurbsV1 { reply_surbs })
    }

    pub fn serialized_len(&self) -> usize {
        v1_reply_surbs_serialised_len(&self.reply_surbs)
    }
}

impl HeartbeatV1 {
    pub fn into_bytes(self) -> Vec<u8> {
        reply_surbs_bytes_v1(&self.additional_reply_surbs).collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        let (additional_reply_surbs, n) = recover_reply_surbs_v1(bytes)?;
        if n != 0 {
            warn!("trailing {n} bytes after v1 heartbeat message");
        }

        Ok(HeartbeatV1 {
            additional_reply_surbs,
        })
    }

    pub fn serialized_len(&self) -> usize {
        v1_reply_surbs_serialised_len(&self.additional_reply_surbs)
    }
}
