// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{
    action_controller::{AckActionSender, Action},
    PendingAcknowledgement, RetransmissionRequestReceiver,
};
use crate::client::real_messages_control::acknowledgement_control::PacketDestination;
use crate::client::real_messages_control::message_handler::{MessageHandler, PreparationError};
use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use crate::client::replies::reply_controller::ReplyControllerSender;
use futures::StreamExt;
use log::*;
use nym_sphinx::chunking::fragment::Fragment;
use nym_sphinx::preparer::PreparedFragment;
use nym_sphinx::{addressing::clients::Recipient, params::PacketType};
use nym_task::{connections::TransmissionLane, TaskClient};
use rand::{CryptoRng, Rng};
use std::sync::{Arc, Weak};

// responsible for packet retransmission upon fired timer
pub(super) struct RetransmissionRequestListener<R> {
    maximum_retransmissions: Option<u32>,
    action_sender: AckActionSender,
    message_handler: MessageHandler<R>,
    request_receiver: RetransmissionRequestReceiver,
    reply_controller_sender: ReplyControllerSender,
    task_client: TaskClient,
}

impl<R> RetransmissionRequestListener<R>
where
    R: CryptoRng + Rng,
{
    pub(super) fn new(
        maximum_retransmissions: Option<u32>,
        action_sender: AckActionSender,
        message_handler: MessageHandler<R>,
        request_receiver: RetransmissionRequestReceiver,
        reply_controller_sender: ReplyControllerSender,
        task_client: TaskClient,
    ) -> Self {
        RetransmissionRequestListener {
            maximum_retransmissions,
            action_sender,
            message_handler,
            request_receiver,
            reply_controller_sender,
            task_client,
        }
    }

    async fn prepare_normal_retransmission_chunk(
        &mut self,
        packet_recipient: Recipient,
        chunk_data: Fragment,
        packet_type: PacketType,
    ) -> Result<PreparedFragment, PreparationError> {
        debug!("retransmitting normal packet...");

        // TODO: Figure out retransmission packet type signaling
        self.message_handler
            .try_prepare_single_chunk_for_sending(packet_recipient, chunk_data, packet_type)
            .await
    }

    async fn on_retransmission_request(
        &mut self,
        weak_timed_out_ack: Weak<PendingAcknowledgement>,
        packet_type: PacketType,
    ) {
        let timed_out_ack = match weak_timed_out_ack.upgrade() {
            Some(timed_out_ack) => timed_out_ack,
            None => {
                debug!("We received an ack JUST as we were about to retransmit [1]");
                return;
            }
        };

        let frag_id = timed_out_ack.message_chunk.fragment_identifier();

        if timed_out_ack.reached_max_retransmissions(self.maximum_retransmissions) {
            warn!("reached maximum number of allowed retransmissions for the packet");
            if let Err(err) = self
                .action_sender
                .unbounded_send(Action::new_remove(frag_id))
            {
                error!("Failed to send remove action to the controller: {err}");
            }
            return;
        }

        let maybe_prepared_fragment = match &timed_out_ack.destination {
            PacketDestination::Anonymous {
                recipient_tag,
                extra_surb_request,
            } => {
                // if this is retransmission for reply, offload it to the dedicated task
                // that deals with all the surbs
                if let Err(err) = self.reply_controller_sender.send_retransmission_data(
                    *recipient_tag,
                    weak_timed_out_ack,
                    *extra_surb_request,
                ) {
                    if !self.task_client.is_shutdown_poll() {
                        error!("Failed to send retransmission data to the reply controller: {err}");
                    }
                }
                return;
            }
            PacketDestination::KnownRecipient(recipient) => {
                self.prepare_normal_retransmission_chunk(
                    **recipient,
                    timed_out_ack.message_chunk.clone(),
                    packet_type,
                )
                .await
            }
        };

        let prepared_fragment = match maybe_prepared_fragment {
            Ok(prepared_fragment) => prepared_fragment,
            Err(err) => {
                warn!("Could not retransmit the packet - {err}");
                // we NEED to start timer here otherwise we will have this guy permanently stuck in memory
                if let Err(err) = self
                    .action_sender
                    .unbounded_send(Action::new_start_timer(frag_id))
                {
                    error!("Failed to send start timer action to the controller: {err}");
                }
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
        if let Err(err) = self
            .action_sender
            .unbounded_send(Action::new_update_pending_ack(frag_id, new_delay))
        {
            if !self.task_client.is_shutdown_poll() {
                error!("Failed to send update pending ack action to the controller: {err}");
            }
        }

        // send to `OutQueueControl` to eventually send to the mix network
        self.message_handler
            .forward_messages(
                vec![RealMessage::new(
                    prepared_fragment.mix_packet,
                    Some(frag_id),
                )],
                TransmissionLane::Retransmission,
            )
            .await
    }

    pub(super) async fn run(&mut self, packet_type: PacketType) {
        debug!("Started RetransmissionRequestListener with graceful shutdown support");

        while !self.task_client.is_shutdown() {
            tokio::select! {
                timed_out_ack = self.request_receiver.next() => match timed_out_ack {
                    Some(timed_out_ack) => self.on_retransmission_request(timed_out_ack, packet_type).await,
                    None => {
                        log::trace!("RetransmissionRequestListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = self.task_client.recv() => {
                    log::trace!("RetransmissionRequestListener: Received shutdown");
                }
            }
        }
        self.task_client.recv_timeout().await;
        log::debug!("RetransmissionRequestListener: Exiting");
    }
}
