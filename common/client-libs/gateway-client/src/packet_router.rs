// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// JS: I personally don't like this name very much, but could not think of anything better.
// I will gladly take any suggestions on how to rename this.

use crate::error::GatewayClientError;
use crate::GatewayPacketRouter;
use futures::channel::mpsc;
use nym_task::TaskClient;

pub type MixnetMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type MixnetMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

pub type AcknowledgementSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type AcknowledgementReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

#[derive(Clone, Debug)]
pub struct PacketRouter {
    ack_sender: AcknowledgementSender,
    mixnet_message_sender: MixnetMessageSender,
    shutdown: TaskClient,
}

impl PacketRouter {
    pub fn new(
        ack_sender: AcknowledgementSender,
        mixnet_message_sender: MixnetMessageSender,
        shutdown: TaskClient,
    ) -> Self {
        PacketRouter {
            ack_sender,
            mixnet_message_sender,
            shutdown,
        }
    }

    pub fn route_mixnet_messages(
        &self,
        received_messages: Vec<Vec<u8>>,
    ) -> Result<(), GatewayClientError> {
        if let Err(err) = self.mixnet_message_sender.unbounded_send(received_messages) {
            // check if the failure is due to the shutdown being in progress and thus the receiver channel
            // having already been dropped
            if self.shutdown.is_shutdown_poll() || self.shutdown.is_dummy() {
                // This should ideally not happen, but it's ok
                log::warn!("Failed to send mixnet messages due to receiver task shutdown");
                return Err(GatewayClientError::ShutdownInProgress);
            }
            // This should never happen during ordinary operation the way it's currently used.
            // Abort to be on the safe side
            panic!("Failed to send mixnet message: {err}");
        }
        Ok(())
    }

    pub fn route_acks(&self, received_acks: Vec<Vec<u8>>) -> Result<(), GatewayClientError> {
        if let Err(err) = self.ack_sender.unbounded_send(received_acks) {
            // check if the failure is due to the shutdown being in progress and thus the receiver channel
            // having already been dropped
            if self.shutdown.is_shutdown_poll() || self.shutdown.is_dummy() {
                // This should ideally not happen, but it's ok
                log::warn!("Failed to send acks due to receiver task shutdown");
                return Err(GatewayClientError::ShutdownInProgress);
            }
            // This should never happen during ordinary operation the way it's currently used.
            // Abort to be on the safe side
            panic!("Failed to send acks: {err}");
        }
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.shutdown.disarm();
    }
}

impl GatewayPacketRouter for PacketRouter {
    type Error = GatewayClientError;

    // note: this trait tries to decide whether a given message is an ack or a data message

    fn route_mixnet_messages(&self, received_messages: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        self.route_mixnet_messages(received_messages)
    }

    fn route_acks(&self, received_acks: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        self.route_acks(received_acks)
    }
}
