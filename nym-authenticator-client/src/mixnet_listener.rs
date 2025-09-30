// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use futures::StreamExt;
use nym_sdk::mixnet::{InputMessage, MixnetClient, MixnetMessageSender, ReconstructedMessage};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub type MixnetMessageBroadcastSender = broadcast::Sender<Arc<ReconstructedMessage>>;
pub type MixnetMessageBroadcastReceiver = broadcast::Receiver<Arc<ReconstructedMessage>>;
pub type MixnetMessageInputSender = mpsc::Sender<InputMessage>;
pub type MixnetMessageInputReceiver = mpsc::Receiver<InputMessage>; // This could be another type, to abstract the mixnet message creation to here

// The AuthClientsMixnetListener listens to mixnet messages and rebroadcasts them to the
// AuthClients, or whoever else is interested.
// It also manages the message input for the mixnet so it can keep the sole ownership of the MixnetClient
//
// NOTE: this is potentially bit wasteful. Ideally we should have proper channels where the
// recipient only gets messages they're interested in.
pub struct AuthClientMixnetListener {
    // The mixnet client that we're listening to
    mixnet_client: MixnetClient,

    // Broadcast channel for the messages that we re-broadcast to the AuthClients
    message_broadcast: MixnetMessageBroadcastSender,

    // Channel for message to send to the mixnet
    input_message_tx: MixnetMessageInputSender, // we keep on to make sure it's open
    input_message_rx: MixnetMessageInputReceiver,

    // Listen to cancel from the outside world
    shutdown_token: CancellationToken,
}

impl AuthClientMixnetListener {
    pub fn new(mixnet_client: MixnetClient, shutdown_token: CancellationToken) -> Self {
        let (message_broadcast, _) = broadcast::channel(100);
        let (input_message_tx, input_message_rx) = mpsc::channel(100);
        Self {
            mixnet_client,
            message_broadcast,
            input_message_tx,
            input_message_rx,
            shutdown_token,
        }
    }

    async fn run(mut self) -> Self {
        let mixnet_cancel_token = self.mixnet_client.cancellation_token();
        self.shutdown_token.run_until_cancelled(async {
            loop {
                tokio::select! {
                    biased;
                    _ = mixnet_cancel_token.cancelled() => {
                        tracing::debug!("AuthClientMixnetListener: mixnet client was shutdown");
                        break;
                    }

                    // Sending loop
                    input_msg = self.input_message_rx.recv() => {
                        match input_msg {
                            None => {
                                tracing::error!("All senders were dropped. It shouldn't happen as we're holding one");
                                break;
                            },
                            Some(mix_msg) => {
                                if let Err(err) = self.mixnet_client.send(mix_msg).await {
                                    tracing::error!("Failed to send mixnet message: {err}");
                                }
                            },
                        }
                    }
                    // Receiving loop
                    msg = self.mixnet_client.next() => {
                        match msg {
                            None => {
                                tracing::error!("Mixnet client stream ended unexpectedly");
                                break;
                            },
                            Some(event) => {
                                if let Err(err) = self.message_broadcast.send(Arc::new(event)) {
                                    tracing::error!("Failed to broadcast mixnet message: {err}");
                                }
                            },

                        }
                    }
                }
            }
            tracing::debug!("AuthClientMixnetListener is shutting down");
        }).await;

        self
    }

    // Disconnects the mixnet client and effectively drop itself, since it doesn't work without one, and reconnecting isn't supported
    pub async fn disconnect_mixnet_client(self) {
        if !self.mixnet_client.cancellation_token().is_cancelled() {
            self.mixnet_client.disconnect().await;
        }
    }

    pub fn start(self) -> AuthClientMixnetListenerHandle {
        let message_broadcast = self.message_broadcast.clone();
        let message_sender = self.input_message_tx.clone();
        // Allows stopping only this, e.g. if we don't need it in the new bandwidth controller
        let cancellation_token = self.shutdown_token.clone();
        let mixnet_cancellation_token = self.mixnet_client.cancellation_token();
        let handle = tokio::spawn(self.run());

        AuthClientMixnetListenerHandle {
            message_broadcast,
            message_sender,
            cancellation_token,
            mixnet_cancellation_token,
            handle,
        }
    }
}

pub struct AuthClientMixnetListenerHandle {
    message_broadcast: MixnetMessageBroadcastSender,
    message_sender: MixnetMessageInputSender,
    cancellation_token: CancellationToken,
    mixnet_cancellation_token: CancellationToken,
    handle: JoinHandle<AuthClientMixnetListener>,
}

impl AuthClientMixnetListenerHandle {
    pub fn mixnet_sender(&self) -> MixnetMessageInputSender {
        self.message_sender.clone()
    }

    pub fn subscribe(&self) -> MixnetMessageBroadcastReceiver {
        self.message_broadcast.subscribe()
    }

    pub fn mixnet_cancel_token(&self) -> CancellationToken {
        self.mixnet_cancellation_token.clone()
    }

    pub async fn stop(self) {
        // If shutdown was externally called, that call is a no-op
        // If we're only stopping this, it is very much needed
        self.cancellation_token.cancel();
        match self.handle.await {
            Ok(auth_client_mixnet_listener) => {
                auth_client_mixnet_listener.disconnect_mixnet_client().await;
            }
            Err(e) => {
                tracing::error!("Error waiting for auth clients mixnet listener to stop: {e}");
            }
        }
    }
}
