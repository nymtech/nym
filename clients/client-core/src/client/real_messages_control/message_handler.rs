// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::real_traffic_stream::{
    BatchRealMessageSender, RealMessage,
};
use crate::client::real_messages_control::{AckActionSender, Action};
use crate::client::replies::reply_storage::{ReceivedReplySurbsMap, SentReplyKeys, UsedSenderTags};
use crate::client::topology_control::{InvalidTopologyError, TopologyAccessor, TopologyReadPermit};
use client_connections::TransmissionLane;
use log::{error, info, warn};
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::{
    AnonymousSenderTag, RepliableMessage, ReplyMessage, SENDER_TAG_SIZE,
};
use nymsphinx::anonymous_replies::{ReplySurb, SurbEncryptionKey};
use nymsphinx::chunking::fragment::Fragment;
use nymsphinx::message::NymMessage;
use nymsphinx::preparer::{MessagePreparer, PreparedFragment};
use rand::{CryptoRng, Rng};
use std::sync::Arc;
use thiserror::Error;
use topology::{NymTopology, NymTopologyError};

// TODO: move that error elsewhere since it seems to be contaminating different files
// TODO2: attempt to unify `InvalidTopologyError` and `NymTopologyError`
#[derive(Debug, Clone, Error)]
pub enum PreparationError {
    #[error(transparent)]
    InvalidTopology(#[from] InvalidTopologyError),

    #[error(transparent)]
    NymTopologyError(#[from] NymTopologyError),

    #[error("The received message cannot be sent using a single reply surb. It ended up getting split into {fragments} fragments.")]
    MessageTooLongForSingleSurb { fragments: usize },

    #[error(
        "Never received any reply SURBs associated with the following sender tag: {sender_tag:?}"
    )]
    UnknownSurbSender { sender_tag: AnonymousSenderTag },

    #[error("Not enough reply SURBs to send the message. We have {available} available and require at least {required}.")]
    NotEnoughSurbs { available: usize, required: usize },
}

impl PreparationError {
    fn return_surbs(self, returned_surbs: Vec<ReplySurb>) -> SurbWrappedPreparationError {
        SurbWrappedPreparationError {
            source: self,
            returned_surbs: Some(returned_surbs),
        }
    }
}

#[derive(Debug, Error)]
#[error("Failed to prepare packets - {source}. {} reply surbs will be returned", .returned_surbs.as_ref().map(|s| s.len()).unwrap_or_default())]
pub struct SurbWrappedPreparationError {
    #[source]
    source: PreparationError,

    returned_surbs: Option<Vec<ReplySurb>>,
}

impl<T> From<T> for SurbWrappedPreparationError
where
    T: Into<PreparationError>,
{
    fn from(err: T) -> Self {
        SurbWrappedPreparationError {
            source: err.into(),
            returned_surbs: None,
        }
    }
}

impl SurbWrappedPreparationError {
    pub(crate) fn return_unused_surbs(
        self,
        surb_storage: &ReceivedReplySurbsMap,
        target: &AnonymousSenderTag,
    ) -> PreparationError {
        if let Some(reply_surbs) = self.returned_surbs {
            surb_storage.insert_surbs(target, reply_surbs)
        }
        self.source
    }
}

#[derive(Clone)]
pub(crate) struct MessageHandler<R> {
    rng: R,
    ack_key: Arc<AckKey>,
    self_address: Recipient,
    message_preparer: MessagePreparer<R>,
    action_sender: AckActionSender,
    real_message_sender: BatchRealMessageSender,
    topology_access: TopologyAccessor,
    reply_key_storage: SentReplyKeys,
    tag_storage: UsedSenderTags,
}

impl<R> MessageHandler<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        rng: R,
        ack_key: Arc<AckKey>,
        self_address: Recipient,
        message_preparer: MessagePreparer<R>,
        action_sender: AckActionSender,
        real_message_sender: BatchRealMessageSender,
        topology_access: TopologyAccessor,
        reply_key_storage: SentReplyKeys,
        tag_storage: UsedSenderTags,
    ) -> Self {
        MessageHandler {
            rng,
            ack_key,
            self_address,
            message_preparer,
            action_sender,
            real_message_sender,
            topology_access,
            reply_key_storage,
            tag_storage,
        }
    }

    fn get_or_create_sender_tag(&mut self, recipient: &Recipient) -> AnonymousSenderTag {
        if let Some(existing) = self.tag_storage.try_get_existing(recipient) {
            info!("we already had sender tag for {recipient}");
            existing
        } else {
            info!("creating new sender tag for {recipient}");
            let mut new_tag = [0u8; SENDER_TAG_SIZE];
            self.rng.fill_bytes(&mut new_tag);
            self.tag_storage.insert_new(recipient, new_tag);
            new_tag
        }
    }

    fn get_topology<'a>(
        &self,
        permit: &'a TopologyReadPermit<'a>,
    ) -> Result<&'a NymTopology, PreparationError> {
        match permit.try_get_valid_topology_ref(&self.self_address, None) {
            Ok(topology_ref) => Ok(topology_ref),
            Err(err) => {
                warn!("Could not process the packet - the network topology is invalid - {err}");
                Err(err.into())
            }
        }
    }

    async fn generate_reply_surbs_with_keys(
        &mut self,
        amount: usize,
    ) -> Result<(Vec<ReplySurb>, Vec<SurbEncryptionKey>), PreparationError> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let reply_surbs = self
            .message_preparer
            .generate_reply_surbs(amount, topology)?;

        let reply_keys = reply_surbs
            .iter()
            .map(|s| *s.encryption_key())
            .collect::<Vec<_>>();

        Ok((reply_surbs, reply_keys))
    }

    pub(crate) async fn try_send_single_surb_message(
        &mut self,
        target: AnonymousSenderTag,
        message: ReplyMessage,
        reply_surb: ReplySurb,
        is_extra_surb_request: bool,
    ) -> Result<(), SurbWrappedPreparationError> {
        let mut fragment = self
            .message_preparer
            .pad_and_split_message(NymMessage::new_reply(message));
        if fragment.len() > 1 {
            // well, it's not a single surb message
            return Err(SurbWrappedPreparationError {
                source: PreparationError::MessageTooLongForSingleSurb {
                    fragments: fragment.len(),
                },
                returned_surbs: Some(vec![reply_surb]),
            });
        }

        let chunk = fragment.pop().unwrap();
        let chunk_clone = chunk.clone();
        let prepared_fragment = self
            .try_prepare_single_reply_chunk_for_sending(reply_surb, chunk_clone)
            .await?;

        let real_messages =
            RealMessage::new(prepared_fragment.mix_packet, chunk.fragment_identifier());
        let delay = prepared_fragment.total_delay;
        let pending_ack =
            PendingAcknowledgement::new_anonymous(chunk, delay, target, is_extra_surb_request);

        let lane = if is_extra_surb_request {
            TransmissionLane::ReplySurbRequest
        } else {
            TransmissionLane::General
        };

        self.forward_messages(vec![real_messages], lane).await;
        self.insert_pending_acks(vec![pending_ack]);
        Ok(())
    }

    pub(crate) async fn try_request_additional_reply_surbs(
        &mut self,
        from: AnonymousSenderTag,
        reply_surb: ReplySurb,
        amount: u32,
    ) -> Result<(), SurbWrappedPreparationError> {
        info!("requesting {amount} reply surbs from {:?}", from);

        let surbs_request = ReplyMessage::new_surb_request_message(self.self_address, amount);
        self.try_send_single_surb_message(from, surbs_request, reply_surb, true)
            .await
    }

    // // TODO: this will require additional argument to make it use different variant of `ReplyMessage`
    pub(crate) fn split_reply_message(&mut self, message: Vec<u8>) -> Vec<Fragment> {
        self.message_preparer
            .pad_and_split_message(NymMessage::new_reply(ReplyMessage::new_data_message(
                message,
            )))
    }

    pub(crate) async fn try_send_reply_chunks(
        &mut self,
        target: AnonymousSenderTag,
        fragments: Vec<Fragment>,
        reply_surbs: Vec<ReplySurb>,
        lane: TransmissionLane,
    ) -> Result<(), SurbWrappedPreparationError> {
        // this should never be reached!
        debug_assert_ne!(
            fragments.len(),
            reply_surbs.len(),
            "attempted to send {} fragments with {} reply surbs",
            fragments.len(),
            reply_surbs.len()
        );

        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Ok(topology) => topology,
            Err(err) => return Err(err.return_surbs(reply_surbs)),
        };

        let mut pending_acks = Vec::with_capacity(fragments.len());
        let mut real_messages = Vec::with_capacity(fragments.len());
        for (fragment, reply_surb) in fragments.into_iter().zip(reply_surbs.into_iter()) {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = fragment.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_reply_chunk_for_sending(chunk_clone, topology, &self.ack_key, reply_surb)
                .unwrap();

            let real_message =
                RealMessage::new(prepared_fragment.mix_packet, fragment.fragment_identifier());
            let delay = prepared_fragment.total_delay;
            let pending_ack = PendingAcknowledgement::new_anonymous(fragment, delay, target, false);

            real_messages.push(real_message);
            pending_acks.push(pending_ack);
        }

        self.forward_messages(real_messages, lane).await;
        self.insert_pending_acks(pending_acks);
        Ok(())
    }

    pub(crate) async fn try_send_plain_message(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        lane: TransmissionLane,
    ) -> Result<(), PreparationError> {
        let message = NymMessage::new_plain(message);
        self.try_split_and_send_non_reply_message(message, recipient, lane)
            .await
    }

    pub(crate) async fn try_split_and_send_non_reply_message(
        &mut self,
        message: NymMessage,
        recipient: Recipient,
        lane: TransmissionLane,
    ) -> Result<(), PreparationError> {
        // TODO: I really dislike existence of this assertion, it implies code has to be re-organised
        debug_assert!(!matches!(message, NymMessage::Reply(_)));

        // TODO2: it's really annoying we have to get topology permit again here due to borrow-checker
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let fragments = self.message_preparer.pad_and_split_message(message);

        let mut pending_acks = Vec::with_capacity(fragments.len());
        let mut real_messages = Vec::with_capacity(fragments.len());
        for fragment in fragments {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = fragment.clone();
            let prepared_fragment = self.message_preparer.prepare_chunk_for_sending(
                chunk_clone,
                topology,
                &self.ack_key,
                &recipient,
            )?;

            let real_message =
                RealMessage::new(prepared_fragment.mix_packet, fragment.fragment_identifier());
            let delay = prepared_fragment.total_delay;
            let pending_ack = PendingAcknowledgement::new_known(fragment, delay, recipient);

            real_messages.push(real_message);
            pending_acks.push(pending_ack);
        }

        self.insert_pending_acks(pending_acks);
        self.forward_messages(real_messages, lane).await;

        Ok(())
    }

    pub(crate) async fn try_send_additional_reply_surbs(
        &mut self,
        recipient: Recipient,
        amount: u32,
    ) -> Result<(), PreparationError> {
        let sender_tag = self.get_or_create_sender_tag(&recipient);
        let (reply_surbs, reply_keys) =
            self.generate_reply_surbs_with_keys(amount as usize).await?;

        let message = NymMessage::new_repliable(RepliableMessage::new_additional_surbs(
            sender_tag,
            reply_surbs,
        ));

        self.try_split_and_send_non_reply_message(
            message,
            recipient,
            TransmissionLane::AdditionalReplySurbs,
        )
        .await?;

        log::trace!("storing {} reply keys", reply_keys.len());
        self.reply_key_storage.insert_multiple(reply_keys);

        Ok(())
    }

    pub(crate) async fn try_send_message_with_reply_surbs(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        num_reply_surbs: u32,
        lane: TransmissionLane,
    ) -> Result<(), SurbWrappedPreparationError> {
        let sender_tag = self.get_or_create_sender_tag(&recipient);
        let (reply_surbs, reply_keys) = self
            .generate_reply_surbs_with_keys(num_reply_surbs as usize)
            .await?;

        let message =
            NymMessage::new_repliable(RepliableMessage::new_data(message, sender_tag, reply_surbs));

        self.try_split_and_send_non_reply_message(message, recipient, lane)
            .await?;

        log::trace!("storing {} reply keys", reply_keys.len());
        self.reply_key_storage.insert_multiple(reply_keys);

        Ok(())
    }

    pub(crate) async fn try_prepare_single_chunk_for_sending(
        &mut self,
        recipient: Recipient,
        chunk: Fragment,
    ) -> Result<PreparedFragment, PreparationError> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let prepared_fragment = self
            .message_preparer
            .prepare_chunk_for_sending(chunk, topology, &self.ack_key, &recipient)
            .unwrap();

        Ok(prepared_fragment)
    }

    pub(crate) async fn try_prepare_single_reply_chunk_for_sending(
        &mut self,
        reply_surb: ReplySurb,
        chunk: Fragment,
    ) -> Result<PreparedFragment, SurbWrappedPreparationError> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Ok(topology) => topology,
            Err(err) => return Err(err.return_surbs(vec![reply_surb])),
        };

        let prepared_fragment = self
            .message_preparer
            .prepare_reply_chunk_for_sending(chunk, topology, &self.ack_key, reply_surb)
            .unwrap();

        Ok(prepared_fragment)
    }

    pub(crate) fn insert_pending_acks(&self, pending_acks: Vec<PendingAcknowledgement>) {
        self.action_sender
            .unbounded_send(Action::new_insert(pending_acks))
            .expect("action control task has died")
    }

    // tells real message sender (with the poisson timer) to send this to the mix network
    pub(super) async fn forward_messages(
        &self,
        messages: Vec<RealMessage>,
        transmission_lane: TransmissionLane,
    ) {
        self.real_message_sender
            .send((messages, transmission_lane))
            .await
            .expect("real message receiver task (OutQueueControl) has died");
    }
}
