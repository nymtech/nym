// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::real_traffic_stream::{
    BatchRealMessageSender, RealMessage,
};
use crate::client::real_messages_control::{AckActionSender, Action};
use crate::client::replies::reply_storage::SentReplyKeys;
use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
use log::{error, info, warn};
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::{AnonymousSenderTag, ReplyMessage};
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::chunking::fragment::{Fragment, FragmentIdentifier};
use nymsphinx::preparer::{MessagePreparer, PreparedFragment};
use nymsphinx::Delay as SphinxDelay;
use rand::{CryptoRng, Rng};
use std::sync::Arc;
use topology::NymTopology;

// TODO: fix those disgusting and lazy Option<()> return types!

#[derive(Clone)]
pub(crate) struct MessageHandler<R> {
    ack_key: Arc<AckKey>,
    self_address: Recipient,
    message_preparer: MessagePreparer<R>,
    action_sender: AckActionSender,
    real_message_sender: BatchRealMessageSender,
    topology_access: TopologyAccessor,
    reply_key_storage: SentReplyKeys,
}

impl<R> MessageHandler<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        ack_key: Arc<AckKey>,
        self_address: Recipient,
        message_preparer: MessagePreparer<R>,
        action_sender: AckActionSender,
        real_message_sender: BatchRealMessageSender,
        topology_access: TopologyAccessor,
        reply_key_storage: SentReplyKeys,
    ) -> Self {
        MessageHandler {
            ack_key,
            self_address,
            message_preparer,
            action_sender,
            real_message_sender,
            topology_access,
            reply_key_storage,
        }
    }

    fn get_topology<'a>(&self, permit: &'a TopologyReadPermit<'a>) -> Option<&'a NymTopology> {
        match permit.try_get_valid_topology_ref(&self.self_address, None) {
            Some(topology_ref) => Some(topology_ref),
            None => {
                warn!("Could not process the packet - the network topology is invalid");
                None
            }
        }
    }

    pub(crate) async fn try_send_single_surb_message(
        &mut self,
        target: AnonymousSenderTag,
        message: ReplyMessage,
        reply_surb: ReplySurb,
        is_extra_surb_request: bool,
    ) -> Result<(), ReplySurb> {
        let mut fragment = self.message_preparer.prepare_and_split_reply(message);
        if fragment.len() > 1 {
            // well, it's not a single surb message
            return Err(reply_surb);
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

        self.forward_messages(vec![real_messages]);
        self.insert_pending_acks(vec![pending_ack]);
        Ok(())
    }

    pub(crate) async fn try_request_additional_reply_surbs(
        &mut self,
        from: AnonymousSenderTag,
        reply_surb: ReplySurb,
        amount: u32,
    ) -> Result<(), ReplySurb> {
        info!("REQUESTING {amount} MORE SURBS!!");
        info!("REQUESTING {amount} MORE SURBS!!");
        info!("REQUESTING {amount} MORE SURBS!!");
        let surbs_request = ReplyMessage::new_surb_request_message(self.self_address, amount);
        self.try_send_single_surb_message(from, surbs_request, reply_surb, true)
            .await
    }

    // TODO: this will require additional argument to make it use different variant of `ReplyMessage`
    pub(crate) fn split_reply_message(&mut self, message: Vec<u8>) -> Vec<Fragment> {
        self.message_preparer
            .prepare_and_split_reply(ReplyMessage::new_data_message(message))
    }

    pub(crate) async fn prepare_reply_message_for_sending(&mut self, message: Vec<u8>) {
        let topology_permit = self.topology_access.get_read_permit().await;
        // let topology = self.get_topology(&topology_permit)?;
    }

    pub(crate) async fn try_send_reply_chunks(
        &mut self,
        target: AnonymousSenderTag,
        fragments: Vec<Fragment>,
        reply_surbs: Vec<ReplySurb>,
    ) -> Result<(), Vec<ReplySurb>> {
        if fragments.len() != reply_surbs.len() {
            // emit an error as this should have never been reached
            error!(
                "attempted to send {} fragments with {} reply surbs",
                fragments.len(),
                reply_surbs.len()
            );
            return Err(reply_surbs);
        }

        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Some(topology) => topology,
            None => return Err(reply_surbs),
        };

        let mut pending_acks = Vec::with_capacity(fragments.len());
        let mut real_messages = Vec::with_capacity(fragments.len());
        for (fragment, reply_surb) in fragments.into_iter().zip(reply_surbs.into_iter()) {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = fragment.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_reply_chunk_for_sending(chunk_clone, topology, reply_surb, &self.ack_key)
                .unwrap();

            let real_message =
                RealMessage::new(prepared_fragment.mix_packet, fragment.fragment_identifier());
            let delay = prepared_fragment.total_delay;
            let pending_ack = PendingAcknowledgement::new_anonymous(fragment, delay, target, false);

            real_messages.push(real_message);
            pending_acks.push(pending_ack);
        }

        self.forward_messages(real_messages);
        self.insert_pending_acks(pending_acks);
        Ok(())
    }

    // TODO: change function signature to better accomodate for 'repliable' messages
    // (for example where you're not sending any plaintext inside)
    pub(crate) async fn try_send_normal_message(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        reply_surbs: u32,
    ) -> Option<()> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        // split the message, attach optional reply surb
        let (fragments, reply_keys) = self
            .message_preparer
            .prepare_and_split_message(message, reply_surbs, topology)
            .expect("somehow the topology was invalid after all!");

        log::trace!("storing {} reply keys", reply_keys.len());
        self.reply_key_storage.insert_multiple(reply_keys);

        let mut pending_acks = Vec::with_capacity(fragments.len());
        let mut real_messages = Vec::with_capacity(fragments.len());
        for fragment in fragments {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = fragment.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(chunk_clone, topology, &self.ack_key, &recipient)
                .unwrap();

            let real_message =
                RealMessage::new(prepared_fragment.mix_packet, fragment.fragment_identifier());
            let delay = prepared_fragment.total_delay;
            let pending_ack = PendingAcknowledgement::new_known(fragment, delay, recipient);

            real_messages.push(real_message);
            pending_acks.push(pending_ack);
        }

        self.insert_pending_acks(pending_acks);
        self.forward_messages(real_messages);

        Some(())
    }

    pub(crate) async fn try_prepare_single_chunk_for_sending(
        &mut self,
        recipient: Recipient,
        chunk: Fragment,
    ) -> Option<PreparedFragment> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let prepared_fragment = self
            .message_preparer
            .prepare_chunk_for_sending(chunk, topology, &self.ack_key, &recipient)
            .unwrap();

        Some(prepared_fragment)
    }

    pub(crate) async fn try_prepare_single_reply_chunk_for_sending(
        &mut self,
        reply_surb: ReplySurb,
        chunk: Fragment,
    ) -> Result<PreparedFragment, ReplySurb> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Some(topology) => topology,
            None => return Err(reply_surb),
        };

        let prepared_fragment = self
            .message_preparer
            .prepare_reply_chunk_for_sending(chunk, topology, reply_surb, &self.ack_key)
            .unwrap();

        Ok(prepared_fragment)
    }

    //
    // fn insert_single_reply_ack(
    //     &self,
    //     message_chunk: Fragment,
    //     delay: SphinxDelay,
    //     recipient_tag: AnonymousSenderTag,
    //     extra_surb_request: bool,
    // ) {
    //     let pending_ack = PendingAcknowledgement::new_anonymous(
    //         message_chunk,
    //         delay,
    //         recipient_tag,
    //         extra_surb_request,
    //     );
    //     self.action_sender
    //         .unbounded_send(Action::new_insert(vec![pending_ack]))
    //         .expect("action control task has died")
    // }

    pub(crate) fn insert_pending_acks(&self, pending_acks: Vec<PendingAcknowledgement>) {
        self.action_sender
            .unbounded_send(Action::new_insert(pending_acks))
            .expect("action control task has died")
    }

    pub(crate) fn update_ack_delay(&self, frag_id: FragmentIdentifier, new_delay: SphinxDelay) {
        self.action_sender
            .unbounded_send(Action::new_update_delay(frag_id, new_delay))
            .expect("action control task has died")
    }

    // tells real message sender (with the poisson timer) to send this to the mix network
    pub(super) fn forward_messages(&self, messages: Vec<RealMessage>) {
        self.real_message_sender
            .unbounded_send(messages)
            .expect("real message receiver task (OutQueueControl) has died")
    }
}
