// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backend::fs_backend::error::StorageError;
use crate::key_storage::UsedReplyKey;
use crate::ReceivedReplySurb;
use nym_crypto::generic_array::typenum::Unsigned;
use nym_crypto::Digest;
use nym_sphinx::addressing::clients::{Recipient, RecipientBytes};
use nym_sphinx::anonymous_replies::encryption_key::EncryptionKeyDigest;
use nym_sphinx::anonymous_replies::requests::{AnonymousSenderTag, SENDER_TAG_SIZE};
use nym_sphinx::anonymous_replies::{
    ReplySurb, ReplySurbWithKeyRotation, SurbEncryptionKey, SurbEncryptionKeySize,
};
use nym_sphinx::params::{ReplySurbKeyDigestAlgorithm, SphinxKeyRotation};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct StoredSenderTag {
    pub recipient: Vec<u8>,
    pub tag: Vec<u8>,
}

impl StoredSenderTag {
    pub fn new(recipient: RecipientBytes, tag: AnonymousSenderTag) -> StoredSenderTag {
        StoredSenderTag {
            recipient: recipient.to_vec(),
            tag: tag.to_bytes().to_vec(),
        }
    }
}

impl TryFrom<StoredSenderTag> for (RecipientBytes, AnonymousSenderTag) {
    type Error = StorageError;

    fn try_from(value: StoredSenderTag) -> Result<Self, Self::Error> {
        let recipient_len = value.recipient.len();
        let Ok(recipient_bytes) = value.recipient.try_into() else {
            return Err(StorageError::CorruptedData {
                details: format!(
                    "the retrieved recipient has length of {recipient_len} while {} was expected",
                    Recipient::LEN
                ),
            });
        };

        let tag_len = value.tag.len();
        let Ok(sender_tag_bytes) = value.tag.try_into() else {
            return Err(StorageError::CorruptedData {
                details: format!(
                    "the retrieved sender tag has length of {tag_len} while {SENDER_TAG_SIZE} was expected",
                ),
            });
        };

        Ok((
            recipient_bytes,
            AnonymousSenderTag::from_bytes(sender_tag_bytes),
        ))
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct StoredReplyKey {
    pub key_digest: Vec<u8>,
    pub reply_key: Vec<u8>,
    pub sent_at: OffsetDateTime,
}

impl StoredReplyKey {
    pub fn new(key_digest: EncryptionKeyDigest, reply_key: UsedReplyKey) -> StoredReplyKey {
        StoredReplyKey {
            key_digest: key_digest.to_vec(),
            reply_key: (*reply_key).to_bytes(),
            sent_at: reply_key.sent_at,
        }
    }
}

impl TryFrom<StoredReplyKey> for (EncryptionKeyDigest, UsedReplyKey) {
    type Error = StorageError;

    fn try_from(value: StoredReplyKey) -> Result<Self, Self::Error> {
        let expected_reply_key_digest_size = ReplySurbKeyDigestAlgorithm::output_size();
        let reply_key_digest_size = value.key_digest.len();

        let Some(digest) = EncryptionKeyDigest::from_exact_iter(value.key_digest) else {
            return Err(StorageError::CorruptedData {
                details: format!(
                    "the reply surb digest has length of {reply_key_digest_size} while {expected_reply_key_digest_size} was expected",
                ),
            });
        };

        let reply_key_len = value.reply_key.len();
        let Ok(reply_key) = SurbEncryptionKey::try_from_bytes(&value.reply_key) else {
            return Err(StorageError::CorruptedData {
                details: format!(
                    "the reply key has length of {reply_key_len} while {} was expected",
                    SurbEncryptionKeySize::USIZE
                ),
            });
        };

        Ok((digest, UsedReplyKey::new(reply_key, value.sent_at)))
    }
}

#[derive(FromRow)]
pub struct StoredSurbSender {
    pub id: i64,
    pub tag: Vec<u8>,
    pub last_sent: OffsetDateTime,
}

impl StoredSurbSender {
    pub fn new(tag: AnonymousSenderTag, last_sent: OffsetDateTime) -> Self {
        StoredSurbSender {
            // for the purposes of STORING data,
            // we ignore that field anyway
            id: 0,
            tag: tag.to_bytes().to_vec(),
            last_sent,
        }
    }
}

impl TryFrom<StoredSurbSender> for (AnonymousSenderTag, OffsetDateTime) {
    type Error = StorageError;

    fn try_from(value: StoredSurbSender) -> Result<Self, Self::Error> {
        let tag_len = value.tag.len();
        let Ok(sender_tag_bytes) = value.tag.try_into() else {
            return Err(StorageError::CorruptedData {
                details: format!(
                    "the retrieved sender tag has length of {tag_len} while {SENDER_TAG_SIZE} was expected",
                ),
            });
        };

        Ok((
            AnonymousSenderTag::from_bytes(sender_tag_bytes),
            value.last_sent,
        ))
    }
}

pub struct StoredReplySurb {
    pub reply_surb_sender_id: i64,
    pub reply_surb: Vec<u8>,

    // encodes only whether it's 'even', 'odd' or 'unknown' (default)
    // and not the whole id because that's redundant
    pub encoded_key_rotation: u8,
}

impl StoredReplySurb {
    pub fn new(reply_surb_sender_id: i64, reply_surb: &ReceivedReplySurb) -> Self {
        StoredReplySurb {
            reply_surb_sender_id,
            reply_surb: reply_surb.surb.inner_reply_surb().to_bytes(),
            encoded_key_rotation: reply_surb.key_rotation() as u8,
        }
    }
}

impl TryFrom<StoredReplySurb> for ReplySurbWithKeyRotation {
    type Error = StorageError;

    fn try_from(value: StoredReplySurb) -> Result<Self, Self::Error> {
        let key_rotation =
            SphinxKeyRotation::try_from(value.encoded_key_rotation).map_err(|err| {
                StorageError::CorruptedData {
                    details: format!("stored key rotation was malformed: {err}"),
                }
            })?;

        let reply_surb = ReplySurb::from_bytes(&value.reply_surb).map_err(|err| {
            StorageError::CorruptedData {
                details: format!("failed to recover the reply surb: {err}"),
            }
        })?;

        Ok(reply_surb.with_key_rotation(key_rotation))
    }
}

#[derive(Copy, Clone)]
pub struct ReplySurbStorageMetadata {
    pub min_reply_surb_threshold: u32,
    pub max_reply_surb_threshold: u32,
}

impl ReplySurbStorageMetadata {
    pub fn new(min_reply_surb_threshold: usize, max_reply_surb_threshold: usize) -> Self {
        Self {
            min_reply_surb_threshold: min_reply_surb_threshold as u32,
            max_reply_surb_threshold: max_reply_surb_threshold as u32,
        }
    }
}
