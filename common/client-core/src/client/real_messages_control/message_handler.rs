// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::real_traffic_stream::{
    BatchRealMessageSender, RealMessage,
};
use crate::client::real_messages_control::{AckActionSender, Action};
use crate::client::replies::reply_controller::MaxRetransmissions;
use crate::client::replies::reply_storage::{ReceivedReplySurbsMap, SentReplyKeys, UsedSenderTags};
use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
use nym_client_core_surb_storage::RetrievedReplySurb;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::{AnonymousSenderTag, RepliableMessage, ReplyMessage};
use nym_sphinx::anonymous_replies::ReplySurbWithKeyRotation;
use nym_sphinx::chunking::fragment::{Fragment, FragmentIdentifier};
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_sphinx::preparer::{MessagePreparer, PreparedFragment};
use nym_sphinx::Delay;
use nym_task::connections::TransmissionLane;
use nym_task::TaskClient;
use nym_topology::{NymRouteProvider, NymTopologyError};
use rand::{CryptoRng, Rng};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

// TODO: move that error elsewhere since it seems to be contaminating different files
#[derive(Debug, Error)]
pub enum PreparationError {
    #[error(transparent)]
    NymTopologyError(#[from] NymTopologyError),

    #[error("message too long for a single SURB, splitting into {fragments} fragments.")]
    MessageTooLongForSingleSurb { fragments: usize },

    #[error(
        "not enough reply SURBs to send the message, available: {available} required: {required}."
    )]
    NotEnoughSurbs { available: usize, required: usize },
}

impl PreparationError {
    fn return_surbs(self, returned_surbs: Vec<RetrievedReplySurb>) -> SurbWrappedPreparationError {
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

    returned_surbs: Option<Vec<RetrievedReplySurb>>,
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
            surb_storage.re_insert_reply_surbs(target, reply_surbs)
        }
        self.source
    }
}

#[derive(Clone)]
pub(crate) struct Config {
    /// Key used to decrypt contents of received SURBAcks
    ack_key: Arc<AckKey>,

    /// Address of this client which also represent an address to which all acknowledgements
    /// and surb-based are going to be sent.
    sender_address: Recipient,

    /// Specify whether route selection should be determined by the packet header.
    deterministic_route_selection: bool,

    /// Indicates whether to mix hops or not. If mix hops are enabled, traffic
    /// will be routed as usual, to the entry gateway, through three mix nodes, egressing
    /// through the exit gateway. If mix hops are disabled, traffic will be routed directly
    /// from the entry gateway to the exit gateway, bypassing the mix nodes.
    ///
    /// This overrides the `use_legacy_sphinx_format` setting as reduced mix hops
    /// requires use of the updated SURB packet format.
    disable_mix_hops: bool,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Primary predefined packet size used for the encapsulated messages.
    primary_packet_size: PacketSize,

    /// Optional secondary predefined packet size used for the encapsulated messages.
    secondary_packet_size: Option<PacketSize>,

    /// Specify whether any constructed reply surbs should use the legacy format,
    /// where the payload keys are explicitly attached rather than using the seeds
    use_legacy_sphinx_format: bool,
}

impl Config {
    pub fn new(
        ack_key: Arc<AckKey>,
        sender_address: Recipient,
        average_packet_delay: Duration,
        average_ack_delay: Duration,
        deterministic_route_selection: bool,
        use_legacy_reply_surb_format: bool,
    ) -> Self {
        Config {
            ack_key,
            sender_address,
            deterministic_route_selection,
            average_packet_delay,
            average_ack_delay,
            primary_packet_size: PacketSize::default(),
            secondary_packet_size: None,
            use_legacy_sphinx_format: use_legacy_reply_surb_format,
            disable_mix_hops: false,
        }
    }

    /// Allows setting non-default size of the sphinx packets sent out.
    pub fn with_custom_primary_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.primary_packet_size = packet_size;
        self
    }

    /// Allows setting non-default size of the sphinx packets sent out.
    pub fn with_custom_secondary_packet_size(mut self, packet_size: Option<PacketSize>) -> Self {
        self.secondary_packet_size = packet_size;
        self
    }

    /// Configure whether messages senders using this config should use mix hops or not when sending messages.
    ///
    /// This overrides the `use_legacy_sphinx_format` setting as disabled mix hops
    /// requires use of the updated SURB packet format.
    pub fn disable_mix_hops(mut self, disable_mix_hops: bool) -> Self {
        self.disable_mix_hops = disable_mix_hops;
        self.use_legacy_sphinx_format = false;
        self
    }
}

#[derive(Clone)]
pub(crate) struct FragmentWithMaxRetransmissions {
    pub(crate) fragment: Fragment,
    pub(crate) max_retransmissions: MaxRetransmissions,
}

#[derive(Clone)]
pub(crate) struct MessageHandler<R> {
    config: Config,
    rng: R,
    message_preparer: MessagePreparer<R>,
    action_sender: AckActionSender,
    real_message_sender: BatchRealMessageSender,
    topology_access: TopologyAccessor,
    reply_key_storage: SentReplyKeys,
    tag_storage: UsedSenderTags,
    task_client: TaskClient,
}

impl<R> MessageHandler<R>
where
    R: CryptoRng + Rng,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: Config,
        rng: R,
        action_sender: AckActionSender,
        real_message_sender: BatchRealMessageSender,
        topology_access: TopologyAccessor,
        reply_key_storage: SentReplyKeys,
        tag_storage: UsedSenderTags,
        task_client: TaskClient,
    ) -> Self
    where
        R: Copy,
    {
        let message_preparer = MessagePreparer::new(
            rng,
            config.deterministic_route_selection,
            config.sender_address,
            config.average_packet_delay,
            config.average_ack_delay,
            config.use_legacy_sphinx_format,
            config.disable_mix_hops,
        );
        MessageHandler {
            config,
            rng,
            message_preparer,
            action_sender,
            real_message_sender,
            topology_access,
            reply_key_storage,
            tag_storage,
            task_client,
        }
    }

    pub(crate) fn topology_access_handle(&self) -> &TopologyAccessor {
        &self.topology_access
    }

    fn get_or_create_sender_tag(&mut self, recipient: &Recipient) -> AnonymousSenderTag {
        if let Some(existing) = self.tag_storage.try_get_existing(recipient) {
            trace!("we already had sender tag for {recipient}");
            existing
        } else {
            debug!("creating new sender tag for {recipient}");
            let new_tag = AnonymousSenderTag::new_random(&mut self.rng);
            self.tag_storage.insert_new(recipient, new_tag);
            info!("using {new_tag} for all anonymous messages sent to {recipient}");
            new_tag
        }
    }

    fn get_topology<'a>(
        &self,
        permit: &'a TopologyReadPermit<'a>,
    ) -> Result<&'a NymRouteProvider, PreparationError> {
        match permit.try_get_valid_topology_ref(&self.config.sender_address, None) {
            Ok(topology_ref) => Ok(topology_ref),
            Err(err) => {
                warn!("Could not process the packet - the network topology is invalid - {err}");
                Err(err.into())
            }
        }
    }

    fn optimal_packet_size(&self, msg: &NymMessage) -> PacketSize {
        // if secondary packet was never set, then it's obvious we have to use the primary packet
        let Some(secondary_packet) = self.config.secondary_packet_size else {
            trace!("only primary packet size is available");
            return self.config.primary_packet_size;
        };

        let primary_count = msg.required_packets(self.config.primary_packet_size);
        let secondary_count = msg.required_packets(secondary_packet);

        trace!("This message would require: {primary_count} primary packets or {secondary_count} secondary packets...");
        // if there would be no benefit in using the secondary packet - use the primary (duh)
        if primary_count <= secondary_count {
            trace!("so choosing primary for this message");
            self.config.primary_packet_size
        } else {
            trace!("so choosing secondary for this message");
            secondary_packet
        }
    }

    async fn generate_reply_surbs(
        &mut self,
        amount: usize,
    ) -> Result<Vec<ReplySurbWithKeyRotation>, PreparationError> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let reply_surbs = self.message_preparer.generate_reply_surbs(
            self.config.use_legacy_sphinx_format,
            amount,
            topology,
        )?;

        Ok(reply_surbs)
    }

    pub(crate) async fn try_send_single_surb_message(
        &mut self,
        target: AnonymousSenderTag,
        message: ReplyMessage,
        reply_surb: RetrievedReplySurb,
        is_extra_surb_request: bool,
    ) -> Result<(), SurbWrappedPreparationError> {
        let msg = NymMessage::new_reply(message);
        let packet_size = self.optimal_packet_size(&msg);
        trace!("Using {packet_size} packets for {msg}");

        let mut fragment = self
            .message_preparer
            .pad_and_split_message(msg, packet_size);
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

        let real_messages = RealMessage::new(
            prepared_fragment.mix_packet,
            Some(chunk.fragment_identifier()),
        );
        let delay = prepared_fragment.total_delay;

        // we have to set a maximum number of retransmissions in case we fail to retrieve
        // surbs for a long period of time; we don't want to be stuck constantly resending the data
        let max_retransmissions = Some(10);
        let pending_ack = PendingAcknowledgement::new_anonymous(
            chunk,
            delay,
            target,
            is_extra_surb_request,
            max_retransmissions,
        );

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
        reply_surb: RetrievedReplySurb,
        amount: u32,
    ) -> Result<(), SurbWrappedPreparationError> {
        debug!("requesting {amount} reply SURBs from {from}");

        let surbs_request =
            ReplyMessage::new_surb_request_message(self.config.sender_address, amount);
        self.try_send_single_surb_message(from, surbs_request, reply_surb, true)
            .await
    }

    // // TODO: this will require additional argument to make it use different variant of `ReplyMessage`
    pub(crate) fn split_reply_message(&mut self, message: Vec<u8>) -> Vec<Fragment> {
        let msg = NymMessage::new_reply(ReplyMessage::new_data_message(message));
        let packet_size = self.optimal_packet_size(&msg);
        trace!("Using {packet_size} packets for {msg}");

        self.message_preparer
            .pad_and_split_message(msg, packet_size)
    }

    pub(crate) async fn send_retransmission_reply_chunks(
        &mut self,
        prepared_fragments: Vec<PreparedFragment>,
        lane: TransmissionLane,
    ) {
        let mut real_messages = Vec::with_capacity(prepared_fragments.len());

        for prepared in prepared_fragments {
            self.update_ack_delay(prepared.fragment_identifier, prepared.total_delay);
            real_messages.push(prepared.into())
        }

        self.forward_messages(real_messages, lane).await;
    }

    pub(crate) async fn try_send_reply_chunks_on_lane(
        &mut self,
        target: AnonymousSenderTag,
        fragments: Vec<FragmentWithMaxRetransmissions>,
        reply_surbs: impl IntoIterator<Item = RetrievedReplySurb>,
        lane: TransmissionLane,
    ) -> Result<(), SurbWrappedPreparationError> {
        self.try_send_reply_chunks(
            target,
            fragments.into_iter().map(|f| (lane, f)).collect(),
            reply_surbs,
        )
        .await
    }

    pub(crate) async fn try_send_reply_chunks(
        &mut self,
        target: AnonymousSenderTag,
        fragments: Vec<(TransmissionLane, FragmentWithMaxRetransmissions)>,
        reply_surbs: impl IntoIterator<Item = RetrievedReplySurb>,
    ) -> Result<(), SurbWrappedPreparationError> {
        let prepared_fragments = self
            .prepare_reply_chunks_for_sending(
                fragments.iter().map(|(_, f)| f.fragment.clone()).collect(),
                reply_surbs,
            )
            .await?;

        let mut pending_acks = Vec::with_capacity(fragments.len());
        let mut to_forward: HashMap<_, Vec<_>> = HashMap::new();

        for (raw, prepared) in fragments.into_iter().zip(prepared_fragments.into_iter()) {
            let lane = raw.0;
            let FragmentWithMaxRetransmissions {
                fragment,
                max_retransmissions,
            } = raw.1;

            let real_message =
                RealMessage::new(prepared.mix_packet, Some(prepared.fragment_identifier));
            let delay = prepared.total_delay;
            let pending_ack = PendingAcknowledgement::new_anonymous(
                fragment,
                delay,
                target,
                false,
                max_retransmissions,
            );

            let entry = to_forward.entry(lane).or_default();
            entry.push(real_message);
            pending_acks.push(pending_ack);
        }

        for (lane, real_messages) in to_forward {
            self.forward_messages(real_messages, lane).await;
        }

        self.insert_pending_acks(pending_acks);
        Ok(())
    }

    pub(crate) async fn send_premade_mix_packets(
        &mut self,
        msgs: Vec<RealMessage>,
        lane: TransmissionLane,
    ) {
        self.forward_messages(msgs, lane).await;
    }

    pub(crate) async fn try_send_plain_message(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        lane: TransmissionLane,
        packet_type: PacketType,
        max_retransmissions: Option<u32>,
    ) -> Result<(), PreparationError> {
        let message = NymMessage::new_plain(message);
        self.try_split_and_send_non_reply_message(
            message,
            recipient,
            lane,
            packet_type,
            max_retransmissions,
        )
        .await
    }

    pub(crate) async fn try_split_and_send_non_reply_message(
        &mut self,
        message: NymMessage,
        recipient: Recipient,
        lane: TransmissionLane,
        packet_type: PacketType,
        max_retransmissions: Option<u32>,
    ) -> Result<(), PreparationError> {
        debug!("Sending non-reply message with packet type {packet_type}");
        // TODO: I really dislike existence of this assertion, it implies code has to be re-organised
        debug_assert!(!matches!(message, NymMessage::Reply(_)));

        // TODO2: it's really annoying we have to get topology permit again here due to borrow-checker
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let packet_size = if packet_type == PacketType::Outfox {
            PacketSize::OutfoxRegularPacket
        } else {
            self.optimal_packet_size(&message)
        };
        trace!("Using {packet_size} packets for {message}");
        let fragments = self
            .message_preparer
            .pad_and_split_message(message, packet_size);

        let mut pending_acks = Vec::with_capacity(fragments.len());
        let mut real_messages = Vec::with_capacity(fragments.len());
        debug!("Splitting message into {} fragments", fragments.len());
        for fragment in fragments {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = fragment.clone();
            let prepared_fragment = self.message_preparer.prepare_chunk_for_sending(
                chunk_clone,
                topology,
                &self.config.ack_key,
                &recipient,
                packet_type,
            )?;

            let real_message = RealMessage::new(
                prepared_fragment.mix_packet,
                Some(fragment.fragment_identifier()),
            );
            let delay = prepared_fragment.total_delay;
            let pending_ack =
                PendingAcknowledgement::new_known(fragment, delay, recipient, max_retransmissions);

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
        packet_type: PacketType,
    ) -> Result<(), PreparationError> {
        debug!("Sending additional reply SURBs with packet type {packet_type}");
        let sender_tag = self.get_or_create_sender_tag(&recipient);
        let reply_surbs = self.generate_reply_surbs(amount as usize).await?;

        let reply_keys = reply_surbs
            .iter()
            .map(|s| *s.encryption_key())
            .collect::<Vec<_>>();

        let message = NymMessage::new_repliable(RepliableMessage::new_additional_surbs(
            self.config.use_legacy_sphinx_format,
            sender_tag,
            reply_surbs,
        ));

        // When sending SURBs we want to retransmit
        let max_retransmissions = None;

        self.try_split_and_send_non_reply_message(
            message,
            recipient,
            TransmissionLane::AdditionalReplySurbs,
            packet_type,
            max_retransmissions,
        )
        .await?;

        tracing::trace!("storing {} reply keys", reply_keys.len());
        self.reply_key_storage.insert_multiple(reply_keys);

        Ok(())
    }

    pub(crate) async fn try_send_message_with_reply_surbs(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        num_reply_surbs: u32,
        lane: TransmissionLane,
        packet_type: PacketType,
        max_retransmissions: Option<u32>,
    ) -> Result<(), SurbWrappedPreparationError> {
        debug!("Sending message with reply SURBs with packet type {packet_type}");
        let sender_tag = self.get_or_create_sender_tag(&recipient);
        let reply_surbs = self.generate_reply_surbs(num_reply_surbs as usize).await?;

        let reply_keys = reply_surbs
            .iter()
            .map(|s| *s.encryption_key())
            .collect::<Vec<_>>();

        let message = NymMessage::new_repliable(RepliableMessage::new_data(
            self.config.use_legacy_sphinx_format,
            message,
            sender_tag,
            reply_surbs,
        ));

        self.try_split_and_send_non_reply_message(
            message,
            recipient,
            lane,
            packet_type,
            max_retransmissions,
        )
        .await?;

        tracing::trace!("storing {} reply keys", reply_keys.len());
        self.reply_key_storage.insert_multiple(reply_keys);

        Ok(())
    }

    pub(crate) async fn try_prepare_single_chunk_for_sending(
        &mut self,
        recipient: Recipient,
        chunk: Fragment,
        packet_type: PacketType,
    ) -> Result<PreparedFragment, PreparationError> {
        debug!("Sending single chunk with packet type {packet_type}");
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let prepared_fragment = self.message_preparer.prepare_chunk_for_sending(
            chunk,
            topology,
            &self.config.ack_key,
            &recipient,
            packet_type,
        )?;

        Ok(prepared_fragment)
    }

    pub(crate) async fn prepare_reply_chunks_for_sending(
        &mut self,
        fragments: Vec<Fragment>,
        reply_surbs: impl IntoIterator<Item = RetrievedReplySurb>,
    ) -> Result<Vec<PreparedFragment>, SurbWrappedPreparationError> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Ok(topology) => topology,
            Err(err) => return Err(err.return_surbs(reply_surbs.into_iter().collect())),
        };

        Ok(fragments
            .into_iter()
            .zip(reply_surbs.into_iter())
            .map(|(fragment, reply_surb)| {
                // unwrap here is fine as we know we have a valid topology
                self.message_preparer
                    .prepare_reply_chunk_for_sending(
                        fragment,
                        topology,
                        &self.config.ack_key,
                        reply_surb.into(),
                        PacketType::Mix,
                    )
                    .unwrap()
            })
            .collect())
    }

    pub(crate) async fn try_prepare_single_reply_chunk_for_sending(
        &mut self,
        reply_surb: RetrievedReplySurb,
        chunk: Fragment,
    ) -> Result<PreparedFragment, SurbWrappedPreparationError> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Ok(topology) => topology,
            Err(err) => return Err(err.return_surbs(vec![reply_surb])),
        };

        let prepared_fragment = self.message_preparer.prepare_reply_chunk_for_sending(
            chunk,
            topology,
            &self.config.ack_key,
            reply_surb.into(),
            PacketType::Mix,
        )?;

        Ok(prepared_fragment)
    }

    pub(crate) fn update_ack_delay(&self, id: FragmentIdentifier, new_delay: Delay) {
        if let Err(err) = self
            .action_sender
            .unbounded_send(Action::UpdatePendingAck(id, new_delay))
        {
            if !self.task_client.is_shutdown_poll() {
                error!("Failed to send update action to the controller: {err}");
            }
        }
    }

    pub(crate) fn insert_pending_acks(&self, pending_acks: Vec<PendingAcknowledgement>) {
        if let Err(err) = self
            .action_sender
            .unbounded_send(Action::new_insert(pending_acks))
        {
            if !self.task_client.is_shutdown_poll() {
                error!("Failed to send insert action to the controller: {err}");
            }
        }
    }

    // tells real message sender (with the poisson timer) to send this to the mix network
    pub(crate) async fn forward_messages(
        &self,
        messages: Vec<RealMessage>,
        transmission_lane: TransmissionLane,
    ) {
        if let Err(err) = self
            .real_message_sender
            .send((messages, transmission_lane))
            .await
        {
            if !self.task_client.is_shutdown_poll() {
                error!("Failed to forward messages to the real message sender: {err}");
            }
        }
    }
}
