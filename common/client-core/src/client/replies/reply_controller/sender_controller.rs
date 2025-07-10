// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_controller::Config;
use nym_client_core_surb_storage::{CombinedReplyStorage, SentReplyKeys, UsedSenderTags};
use nym_crypto::aes::cipher::crypto_common::rand_core::CryptoRng;
use nym_sphinx::addressing::Recipient;
use rand::Rng;
use std::cmp::min;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, trace, warn};

/// Reply controller responsible for controlling sender-related part
/// of replies, such as checking if any reply keys are stale
pub struct SenderReplyController<R> {
    config: Config,

    tags_storage: UsedSenderTags,
    sent_reply_keys: SentReplyKeys,
    message_handler: MessageHandler<R>,
}

impl<R> SenderReplyController<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        config: Config,
        storage: &CombinedReplyStorage,
        message_handler: MessageHandler<R>,
    ) -> Self {
        SenderReplyController {
            config,
            tags_storage: storage.tags_storage(),
            sent_reply_keys: storage.key_storage(),
            message_handler,
        }
    }

    pub(crate) async fn handle_surb_request(&mut self, recipient: Recipient, mut amount: u32) {
        // 1. check whether we sent any surbs in the past to this recipient, otherwise
        // they have no business in asking for more
        if !self.tags_storage.exists(&recipient) {
            warn!("{recipient} asked us for reply SURBs even though we never sent them any anonymous messages before!");
            return;
        }

        // 2. check whether the requested amount is within sane range
        if amount
            > self
                .config
                .reply_surbs
                .maximum_allowed_reply_surb_request_size
        {
            warn!("The requested reply surb amount is larger than our maximum allowed ({amount} > {}). Lowering it to a more sane value...", self.config.reply_surbs.maximum_allowed_reply_surb_request_size);
            amount = self
                .config
                .reply_surbs
                .maximum_allowed_reply_surb_request_size;
        }

        // 3. construct and send the surbs away
        // (send them in smaller batches to make the experience a bit smoother
        let mut remaining = amount;
        while remaining > 0 {
            let to_send = min(remaining, 100);
            if let Err(err) = self
                .message_handler
                .try_send_additional_reply_surbs(
                    recipient,
                    to_send,
                    nym_sphinx::params::PacketType::Mix,
                )
                .await
            {
                warn!("failed to send additional surbs to {recipient} - {err}");
            } else {
                trace!("sent {to_send} reply SURBs to {recipient}");
            }

            remaining -= to_send;
        }
    }

    pub(crate) fn inspect_and_clear_stale_data(&self, now: OffsetDateTime) {
        // check reply keys (this applies to SENDER)
        self.sent_reply_keys.retain(|_, reply_key| {
            let diff = now - reply_key.sent_at;
            if diff > self.config.reply_surbs.maximum_reply_key_age {
                let std_diff = Duration::try_from(diff).unwrap_or_default();
                let diff_formatted = humantime::format_duration(std_diff);
                debug!("it's been {diff_formatted} since we created this reply key. it's probably never going to get used, so we're going to purge it...");
                false
            } else {
                true
            }
        });
    }
}
