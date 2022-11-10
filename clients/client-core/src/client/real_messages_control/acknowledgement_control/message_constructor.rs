// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: move it elsewhere, I just extracted it to different module
// as it seems multiple structs were already depending on this exact structure
// 
// use super::action_controller::{Action, ActionSender};
// use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
// use crate::client::real_messages_control::real_traffic_stream::{
//     BatchRealMessageSender, RealMessage,
// };
// use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
// use log::warn;
// use nymsphinx::acknowledgements::AckKey;
// use nymsphinx::addressing::clients::Recipient;
// use nymsphinx::chunking::fragment::{Fragment, FragmentIdentifier};
// use nymsphinx::preparer::{MessagePreparer, PreparedFragment};
// use nymsphinx::Delay as SphinxDelay;
// use rand::{CryptoRng, Rng};
// use std::sync::Arc;
// use topology::NymTopology;
// 
// pub(super) struct MessageConstructor<R>
// where
//     R: CryptoRng + Rng,
// {
//     ack_key: Arc<AckKey>,
//     ack_recipient: Recipient,
//     message_preparer: MessagePreparer<R>,
//     action_sender: ActionSender,
//     real_message_sender: BatchRealMessageSender,
//     topology_access: TopologyAccessor,
// }
// 
// impl<R> MessageConstructor<R>
// where
//     R: CryptoRng + Rng,
// {
//     fn get_topology<'a>(&self, permit: &'a TopologyReadPermit<'a>) -> Option<&'a NymTopology> {
//         match permit.try_get_valid_topology_ref(&self.ack_recipient, None) {
//             Some(topology_ref) => Some(topology_ref),
//             None => {
//                 warn!("Could not process the packet - the network topology is invalid");
//                 None
//             }
//         }
//     }
// 
//     pub(super) async fn prepare_normal_chunks_for_sending(
//         &mut self,
//         recipient: Recipient,
//         chunks: Vec<Fragment>,
//         is_fresh: bool,
//     ) -> Option<Vec<(PreparedFragment, FragmentIdentifier)>> {
//         let topology_permit = self.topology_access.get_read_permit().await;
//         let topology = self.get_topology(&topology_permit)?;
// 
//         let mut pending_acks = Vec::with_capacity(chunks.len());
//         let mut prepared_messages = Vec::with_capacity(chunks.len());
//         for message_chunk in chunks {
//             // we need to clone it because we need to keep it in memory in case we had to retransmit
//             // it. And then we'd need to recreate entire ACK again.
//             let chunk_clone = message_chunk.clone();
//             let prepared_fragment = self
//                 .message_preparer
//                 .prepare_chunk_for_sending(chunk_clone, topology, &self.ack_key, &recipient)
//                 .unwrap();
// 
//             prepared_messages.push(RealMessage::new(
//                 prepared_fragment.mix_packet,
//                 message_chunk.fragment_identifier(),
//             ));
// 
//             if is_fresh {
//                 pending_acks.push(PendingAcknowledgement::new(
//                     message_chunk,
//                     prepared_fragment.total_delay,
//                     recipient,
//                 ));
//             }
//         }
// 
//         // if it's the first time we're sending the packet, insert ack info
//         // otherwise, we're going to update the existing delay information
//         // (but outside of this method as we have to check for reference count first
//         if is_fresh {
//             // tells the controller to put this into the hashmap
//             self.insert_pending_acks(pending_acks)
//         }
// 
//         Some(prepared_messages)
//     }
// 
//     pub(super) fn insert_pending_acks(&self, pending_acks: Vec<PendingAcknowledgement>) {
//         self.action_sender
//             .unbounded_send(Action::new_insert(pending_acks))
//             .expect("action control task has died")
//     }
// 
//     pub(super) fn update_ack_delay(&self, frag_id: FragmentIdentifier, new_delay: SphinxDelay) {
//         self.action_sender
//             .unbounded_send(Action::new_update_delay(frag_id, new_delay))
//             .expect("action control task has died")
//     }
// 
//     pub(super) fn forward_messages(&self, messages: Vec<RealMessage>) {
//         self.real_message_sender
//             .unbounded_send(messages)
//             .expect("real message receiver task (OutQueueControl) has died")
//     }
// }
