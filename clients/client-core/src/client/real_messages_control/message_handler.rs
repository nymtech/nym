// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::real_traffic_stream::{
    BatchRealMessageSender, RealMessage,
};
use crate::client::real_messages_control::{AckActionSender, Action};
use crate::client::replies::reply_storage::SentReplyKeys;
use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
use log::{error, warn};
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::ReplyMessage;
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
        message: ReplyMessage,
        reply_surb: ReplySurb,
    ) -> Result<(), ReplySurb> {
        // TODO: this should really be more streamlined as we use the same pattern in multiple places
        let mut fragment = self.message_preparer.prepare_and_split_reply(message);
        if fragment.len() > 1 {
            // well, it's not a single surb message
            return Err(reply_surb);
        }

        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match self.get_topology(&topology_permit) {
            Some(topology) => topology,
            None => return Err(reply_surb),
        };

        let chunk = fragment.pop().unwrap();
        let chunk_clone = chunk.clone();
        let prepared_fragment = self
            .message_preparer
            .prepare_reply_chunk_for_sending(chunk_clone, topology, reply_surb, &self.ack_key)
            .unwrap();

        // TODO: ack and retransmission for the sucker...

        let real_messages =
            RealMessage::new(prepared_fragment.mix_packet, chunk.fragment_identifier());

        self.forward_messages(vec![real_messages]);
        Ok(())
    }

    pub(crate) async fn try_request_additional_reply_surbs(
        &mut self,
        reply_surb: ReplySurb,
        amount: u32,
    ) -> Result<(), ReplySurb> {
        let surbs_request = ReplyMessage::new_surb_request_message(self.self_address, amount);
        self.try_send_single_surb_message(surbs_request, reply_surb)
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

        let mut real_messages = Vec::with_capacity(reply_surbs.len());
        for (fragment, reply_surb) in fragments.into_iter().zip(reply_surbs.into_iter()) {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = fragment.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_reply_chunk_for_sending(chunk_clone, topology, reply_surb, &self.ack_key)
                .unwrap();

            real_messages.push(RealMessage::new(
                prepared_fragment.mix_packet,
                fragment.fragment_identifier(),
            ));

            // TODO: deal with retransmission and acks here
        }

        self.forward_messages(real_messages);
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
        let fragments = self
            .prepare_normal_message_for_sending(recipient, message, reply_surbs, true)
            .await?;
        let real_messages = fragments.into_iter().map(Into::into).collect();
        self.forward_messages(real_messages);

        Some(())
    }

    // TODO: change function signature to better accomodate for 'repliable' messages
    // (for example where you're not sending any plaintext inside)
    pub(crate) async fn prepare_normal_message_for_sending(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        reply_surbs: u32,
        is_fresh: bool,
    ) -> Option<Vec<(PreparedFragment, FragmentIdentifier)>> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        // split the message, attach optional reply surb
        let (split_message, reply_keys) = self
            .message_preparer
            .prepare_and_split_message(message, reply_surbs, topology)
            .expect("somehow the topology was invalid after all!");

        drop(topology_permit);

        log::info!("storing {} reply keys", reply_keys.len());
        self.reply_key_storage.insert_multiple(reply_keys);
        // self.pr

        self.prepare_normal_chunks_for_sending(recipient, split_message, is_fresh)
            .await
    }

    pub(crate) async fn prepare_normal_chunks_for_sending(
        &mut self,
        recipient: Recipient,
        chunks: Vec<Fragment>,
        is_fresh: bool,
    ) -> Option<Vec<(PreparedFragment, FragmentIdentifier)>> {
        // TODO: optimisation: if this is called from `prepare_normal_message_for_sending`,
        // somehow try to avoid having to re-acquire the topology permit
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let mut pending_acks = Vec::with_capacity(chunks.len());
        let mut prepared_messages = Vec::with_capacity(chunks.len());
        for message_chunk in chunks {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = message_chunk.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(chunk_clone, topology, &self.ack_key, &recipient)
                .unwrap();

            let total_delay = prepared_fragment.total_delay;

            prepared_messages.push((prepared_fragment, message_chunk.fragment_identifier()));

            if is_fresh {
                pending_acks.push(PendingAcknowledgement::new(
                    message_chunk,
                    total_delay,
                    recipient,
                ));
            }
        }

        // // if it's the first time we're sending the packet, insert ack info
        // // otherwise, we're going to update the existing delay information
        // // (but outside of this method as we have to check for reference count first)
        if is_fresh {
            // tells the controller to put this into the hashmap
            self.insert_pending_acks(pending_acks)
        }

        Some(prepared_messages)
    }

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
