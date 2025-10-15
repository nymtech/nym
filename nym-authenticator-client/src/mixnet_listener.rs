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

// Spawn a task that listens to mixnet messages and rebroadcasts them to the
// AuthClients, or whoever else is interested.
// It also manages the message input for the mixnet so it can keep the sole ownership of the MixnetClient
//
// NOTE: this is potentially bit wasteful. Ideally we should have proper channels where the
// recipient only gets messages they're interested in.
pub fn spawn(
    mut mixnet_client: MixnetClient,
    shutdown_token: CancellationToken,
) -> AuthClientMixnetListenerHandle {
    // Broadcast channel for the messages that we re-broadcast to the AuthClients
    let (message_broadcast, _) = broadcast::channel(100);
    // Channel for message to send to the mixnet
    let (input_message_tx, mut input_message_rx) = mpsc::channel(100);

    let cloned_message_broadcast = message_broadcast.clone();
    let cloned_message_sender = input_message_tx.clone();
    let child_shutdown_token = shutdown_token.child_token();

    let join_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = child_shutdown_token.cancelled() => {
                    tracing::debug!("AuthClientMixnetListener: received shutdown");
                    // Disconnect mixnet which should send forget_me or remember_me if needed.
                    mixnet_client.disconnect().await;
                    break;
                }

                // Sending loop
                input_msg = input_message_rx.recv() => {
                    match input_msg {
                        None => {
                            tracing::error!("All senders were dropped. It shouldn't happen as we're holding one");
                            break;
                        },
                        Some(mix_msg) => {
                            if let Err(err) = mixnet_client.send(mix_msg).await {
                                tracing::error!("Failed to send mixnet message: {err}");
                            }
                        },
                    }
                }
                // Receiving loop
                msg = mixnet_client.next() => {
                    match msg {
                        None => {
                            tracing::error!("Mixnet client stream ended unexpectedly");
                            break;
                        },
                        Some(event) => {
                            if let Err(err) = message_broadcast.send(Arc::new(event)) {
                                tracing::error!("Failed to broadcast mixnet message: {err}");
                            }
                        },

                    }
                }
            }
        }
        tracing::debug!("AuthClientMixnetListener is shutting down");
    });

    AuthClientMixnetListenerHandle {
        message_broadcast: cloned_message_broadcast,
        message_sender: cloned_message_sender,
        join_handle,
    }
}

pub struct AuthClientMixnetListenerHandle {
    message_broadcast: MixnetMessageBroadcastSender,
    message_sender: MixnetMessageInputSender,
    join_handle: JoinHandle<()>,
}

impl AuthClientMixnetListenerHandle {
    pub fn mixnet_sender(&self) -> MixnetMessageInputSender {
        self.message_sender.clone()
    }

    pub fn subscribe(&self) -> MixnetMessageBroadcastReceiver {
        self.message_broadcast.subscribe()
    }

    /// Join on listener handle to wait until it stops
    ///
    /// Important: Use cancellation token to initiate the shutdown
    pub async fn join(self) {
        if let Err(e) = self.join_handle.await {
            tracing::error!("Error waiting for auth clients mixnet listener to stop: {e}");
        }
    }
}
