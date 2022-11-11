// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use super::PendingAcknowledgement;
use super::RetransmissionRequestReceiver;
use crate::client::real_messages_control::acknowledgement_control::PacketDestination;
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use crate::client::replies::reply_storage::ReceivedReplySurbsMap;
use futures::StreamExt;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::chunking::fragment::Fragment;
use nymsphinx::preparer::PreparedFragment;
use rand::{CryptoRng, Rng};
use std::sync::{Arc, Weak};

// that should be moved into config
#[deprecated]
const SURBS_TO_REQUEST: u32 = 20;

// responsible for packet retransmission upon fired timer
pub(super) struct RetransmissionRequestListener<R> {
    action_sender: AckActionSender,
    message_handler: MessageHandler<R>,
    request_receiver: RetransmissionRequestReceiver,

    // we're holding this for the purposes of retransmitting dropped reply message, but perhaps
    // this work should be offloaded to the `ToBeNamedPendingReplyController`?
    received_reply_surbs: ReceivedReplySurbsMap,
}

impl<R> RetransmissionRequestListener<R>
where
    R: CryptoRng + Rng,
{
    pub(super) fn new(
        action_sender: AckActionSender,
        message_handler: MessageHandler<R>,
        request_receiver: RetransmissionRequestReceiver,
        received_reply_surbs: ReceivedReplySurbsMap,
    ) -> Self {
        RetransmissionRequestListener {
            action_sender,
            message_handler,
            request_receiver,
            received_reply_surbs,
        }
    }

    async fn prepare_normal_retransmission_chunk(
        &mut self,
        packet_recipient: Recipient,
        chunk_data: Fragment,
    ) -> Option<PreparedFragment> {
        debug!("retransmitting normal packet...");

        self.message_handler
            .try_prepare_single_chunk_for_sending(packet_recipient, chunk_data)
            .await
    }

    async fn prepare_reply_retransmission_chunk(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        extra_surb_request: bool,
        chunk_data: Fragment,
    ) -> Option<PreparedFragment> {
        error!("retransmitting reply packet...");

        // TODO: collapse that if monstrosity...
        let maybe_reply_surb = if extra_surb_request {
            info!("retransmitting packet requesting for additional surbs - it MUST get through...");
            self.received_reply_surbs
                .get_reply_surb_ignoring_threshold(&recipient_tag)?
                .0
        } else {
            let (maybe_surb, surbs_left) =
                self.received_reply_surbs.get_reply_surb(&recipient_tag)?;
            if self.received_reply_surbs.below_threshold(surbs_left) {
                // if we're running low on surbs, we should request more
                if let Some(another_surb) = self
                    .received_reply_surbs
                    .get_reply_surb_ignoring_threshold(&recipient_tag)?
                    .0
                {
                    if let Err(returned_surb) = self
                        .message_handler
                        .try_request_additional_reply_surbs(
                            recipient_tag,
                            another_surb,
                            SURBS_TO_REQUEST,
                        )
                        .await
                    {
                        warn!("we failed to ask for more surbs...");
                        self.received_reply_surbs
                            .insert_surb(&recipient_tag, returned_surb)
                    }
                }
            }

            maybe_surb
        };

        let Some(reply_surb) = maybe_reply_surb else {
            warn!("we run out of reply surbs for {:?} to retransmit our dropped message...", recipient_tag);
            return None
        };

        match self
            .message_handler
            .try_prepare_single_reply_chunk_for_sending(reply_surb, chunk_data)
            .await
        {
            Ok(prepared_fragment) => Some(prepared_fragment),
            Err(returned_surb) => {
                self.received_reply_surbs
                    .insert_surb(&recipient_tag, returned_surb);
                None
            }
        }
    }

    async fn on_retransmission_request(&mut self, timed_out_ack: Weak<PendingAcknowledgement>) {
        let timed_out_ack = match timed_out_ack.upgrade() {
            Some(timed_out_ack) => timed_out_ack,
            None => {
                debug!("We received an ack JUST as we were about to retransmit [1]");
                return;
            }
        };

        let chunk_clone = timed_out_ack.message_chunk.clone();
        let frag_id = chunk_clone.fragment_identifier();

        let maybe_prepared_fragment = match &timed_out_ack.destination {
            PacketDestination::Anonymous {
                recipient_tag,
                extra_surb_request,
            } => {
                self.prepare_reply_retransmission_chunk(
                    *recipient_tag,
                    *extra_surb_request,
                    chunk_clone,
                )
                .await
            }
            PacketDestination::KnownRecipient(recipient) => {
                self.prepare_normal_retransmission_chunk(*recipient, chunk_clone)
                    .await
            }
        };

        let Some(prepared_fragment) = maybe_prepared_fragment else {
            warn!("Could not retransmit the packet - the network topology is invalid");
            // we NEED to start timer here otherwise we will have this guy permanently stuck in memory
            self.action_sender
                .unbounded_send(Action::new_start_timer(frag_id))
                .unwrap();
            return;
        };

        // if we have the ONLY strong reference to the ack data, it means it was removed from the
        // pending acks
        if Arc::strong_count(&timed_out_ack) == 1 {
            // while we were messing with topology, wrapping data in sphinx, etc. we actually received
            // this ack after all! no need to retransmit then
            debug!("We received an ack JUST as we were about to retransmit [2]");
            return;
        }
        // we no longer need the reference - let's drop it so that if somehow `UpdateTimer` action
        // reached the controller before this function terminated, the controller would not panic.
        drop(timed_out_ack);
        let new_delay = prepared_fragment.total_delay;

        // We know this update will be reflected by the `StartTimer` Action performed when this
        // message is sent through the mix network.
        // Reason being: UpdateTimer is now pushed onto the Action queue and `StartTimer` will
        // only be pushed when the below `RealMessage` (which we are about to create)
        // is sent to the `OutQueueControl` and has gone through its internal queue
        // with the additional poisson delay.
        // And since Actions are executed in order `UpdateTimer` will HAVE TO be executed before `StartTimer`
        self.action_sender
            .unbounded_send(Action::new_update_delay(frag_id, new_delay))
            .unwrap();

        // send to `OutQueueControl` to eventually send to the mix network
        self.message_handler.forward_messages(vec![RealMessage::new(
            prepared_fragment.mix_packet,
            frag_id,
        )])
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started RetransmissionRequestListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                timed_out_ack = self.request_receiver.next() => match timed_out_ack {
                    Some(timed_out_ack) => self.on_retransmission_request(timed_out_ack).await,
                    None => {
                        log::trace!("RetransmissionRequestListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv() => {
                    log::trace!("RetransmissionRequestListener: Received shutdown");
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("RetransmissionRequestListener: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn run(&mut self) {
        debug!("Started RetransmissionRequestListener without graceful shutdown support");

        while let Some(timed_out_ack) = self.request_receiver.next().await {
            self.on_retransmission_request(timed_out_ack).await;
        }
    }
}
