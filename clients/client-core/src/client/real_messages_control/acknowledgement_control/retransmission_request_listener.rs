// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{
    action_controller::{AckActionSender, Action},
    PendingAcknowledgement, RetransmissionRequestReceiver,
};
use crate::client::real_messages_control::acknowledgement_control::PacketDestination;
use crate::client::real_messages_control::message_handler::{MessageHandler, PreparationError};
use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use crate::client::replies::reply_storage::ReceivedReplySurbsMap;
use client_connections::TransmissionLane;
use futures::StreamExt;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::chunking::fragment::Fragment;
use nymsphinx::preparer::PreparedFragment;
use rand::{CryptoRng, Rng};
use std::sync::{Arc, Weak};

// responsible for packet retransmission upon fired timer
pub(super) struct RetransmissionRequestListener<R> {
    action_sender: AckActionSender,
    message_handler: MessageHandler<R>,
    request_receiver: RetransmissionRequestReceiver,

    // we're holding this for the purposes of retransmitting dropped reply message, but perhaps
    // this work should be offloaded to the `ToBeNamedPendingReplyController`?
    received_reply_surbs: ReceivedReplySurbsMap,

    reply_surb_request_size: u32,
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
        reply_surb_request_size: u32,
    ) -> Self {
        RetransmissionRequestListener {
            action_sender,
            message_handler,
            request_receiver,
            received_reply_surbs,
            reply_surb_request_size,
        }
    }

    async fn prepare_normal_retransmission_chunk(
        &mut self,
        packet_recipient: Recipient,
        chunk_data: Fragment,
    ) -> Result<PreparedFragment, PreparationError> {
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
    ) -> Result<PreparedFragment, PreparationError> {
        error!("retransmitting reply packet...");

        let surbs_left = self.received_reply_surbs.available_surbs(&recipient_tag);
        println!("{surbs_left} surbs left");

        // if this is retransmission for obtaining additional reply surbs,
        // we can dip below the storage threshold
        let (maybe_reply_surb, surbs_left) = if extra_surb_request {
            self.received_reply_surbs
                .get_reply_surb_ignoring_threshold(&recipient_tag)
        } else {
            self.received_reply_surbs.get_reply_surb(&recipient_tag)
        }
        .ok_or(PreparationError::UnknownSurbSender {
            sender_tag: recipient_tag,
        })?;

        // but if it wasn't a retransmission for obtaining additional reply surbs
        // and we're now below threshold, attempt to request additional surbs
        if !extra_surb_request && self.received_reply_surbs.below_threshold(surbs_left) {
            // if we're running low on surbs, we should request more (unless we've already requested them)
            let pending_reception = self.received_reply_surbs.pending_reception(&recipient_tag);

            if pending_reception < self.reply_surb_request_size {
                info!("requesting surbs from retransmission handler");

                // TODO: is this logic for surb request possibly shared with other parts already?
                if let Some(another_surb) = self
                    .received_reply_surbs
                    .get_reply_surb_ignoring_threshold(&recipient_tag)
                    .ok_or(PreparationError::UnknownSurbSender {
                        sender_tag: recipient_tag,
                    })?
                    .0
                {
                    if let Err(err) = self
                        .message_handler
                        .try_request_additional_reply_surbs(
                            recipient_tag,
                            another_surb,
                            self.reply_surb_request_size,
                        )
                        .await
                    {
                        let err =
                            err.return_unused_surbs(&self.received_reply_surbs, &recipient_tag);
                        warn!("we failed to ask for more surbs... - {err}");
                        // TODO: should we return here instead?
                    }
                    self.received_reply_surbs
                        .increment_pending_reception(&recipient_tag, self.reply_surb_request_size)
                        .ok_or(PreparationError::UnknownSurbSender {
                            sender_tag: recipient_tag,
                        })?;
                }
            }
        }

        let Some(reply_surb) = maybe_reply_surb else {
            warn!("we run out of reply surbs for {:?} to retransmit our dropped message...", recipient_tag);
            return Err(PreparationError::NotEnoughSurbs { available: 0, required: 1 })
        };

        match self
            .message_handler
            .try_prepare_single_reply_chunk_for_sending(reply_surb, chunk_data)
            .await
        {
            Ok(prepared_fragment) => Ok(prepared_fragment),
            Err(err) => {
                let err = err.return_unused_surbs(&self.received_reply_surbs, &recipient_tag);
                warn!("failed to prepare message for retransmission - {err}",);
                Err(err)
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
                // TODO: preserve err info
                self.prepare_normal_retransmission_chunk(*recipient, chunk_clone)
                    .await
            }
        };

        let prepared_fragment = match maybe_prepared_fragment {
            Ok(prepared_fragment) => prepared_fragment,
            Err(err) => {
                warn!("Could not retransmit the packet - {err}");
                // we NEED to start timer here otherwise we will have this guy permanently stuck in memory

                // TODO: purge the entry from memory if it was an ack for reply packet and we're out of surbs
                // self.action_sender
                //     .unbounded_send(Action::new_remove(frag_id))
                //     .unwrap();

                self.action_sender
                    .unbounded_send(Action::new_start_timer(frag_id))
                    .unwrap();
                return;
            }
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
        self.message_handler
            .forward_messages(
                vec![RealMessage::new(prepared_fragment.mix_packet, frag_id)],
                TransmissionLane::Retransmission,
            )
            .await
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
