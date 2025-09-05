// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// To remove with the Registration Client PR
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use futures::StreamExt;
use nym_sdk::mixnet::{MixnetClient, ReconstructedMessage};
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::AuthenticatorMixnetClient;

pub type SharedMixnetClient = Arc<tokio::sync::Mutex<Option<MixnetClient>>>;
pub type MixnetMessageBroadcastSender = broadcast::Sender<Arc<ReconstructedMessage>>;
pub type MixnetMessageBroadcastReceiver = broadcast::Receiver<Arc<ReconstructedMessage>>;

// The AuthClientsMixnetListener listens to mixnet messages and rebroadcasts them to the
// AuthClients, or whoever else is interested.
// While it is running, it has a lock on the shared mixnet client. This is the reason it's
// designed to be able to start and stop, so that the lock can be released when it's not needed.
//
// NOTE: this is potentially bit wasteful. Ideally we should have proper channels where the
// recipient only gets messages they're interested in.
pub struct AuthClientMixnetListener {
    // The shared mixnet client that we're listening to
    mixnet_client: SharedMixnetClient,

    // Broadcast channel for the messages that we re-broadcast to the AuthClients
    message_broadcast: MixnetMessageBroadcastSender,

    // Listen to cancel from the outside world
    shutdown_token: CancellationToken,
}

impl AuthClientMixnetListener {
    pub fn new(mixnet_client: SharedMixnetClient, shutdown_token: CancellationToken) -> Self {
        let (message_broadcast, _) = broadcast::channel(100);
        Self {
            mixnet_client,
            message_broadcast,
            shutdown_token,
        }
    }

    pub fn subscribe(&self) -> MixnetMessageBroadcastReceiver {
        self.message_broadcast.subscribe()
    }

    async fn run(self) {
        let mut mixnet_client = self.mixnet_client.lock().await.take().unwrap();
        self.shutdown_token
            .run_until_cancelled(async {
                while let Some(event) = mixnet_client.next().await {
                    if let Err(err) = self.message_broadcast.send(Arc::new(event)) {
                        tracing::error!("Failed to broadcast mixnet message: {err}");
                    }
                }
                tracing::error!("Mixnet client stream ended unexpectedly");
            })
            .await;
        self.mixnet_client.lock().await.replace(mixnet_client);
    }

    pub fn start(self) -> AuthClientMixnetListenerHandle {
        let mixnet_client = self.mixnet_client.clone();
        let message_broadcast = self.message_broadcast.clone();
        let handle = tokio::spawn(self.run());

        AuthClientMixnetListenerHandle {
            mixnet_client,
            message_broadcast,
            handle,
        }
    }
}

pub struct AuthClientMixnetListenerHandle {
    mixnet_client: SharedMixnetClient,
    message_broadcast: MixnetMessageBroadcastSender,
    handle: JoinHandle<()>,
}

impl AuthClientMixnetListenerHandle {
    /// Returns new `AuthClient` or `None` if `MixnetClient` is already moved from shared reference.
    pub async fn new_auth_client(&self) -> Option<AuthenticatorMixnetClient> {
        let mixnet_client_guard = self.mixnet_client.lock().await;
        let mixnet_client_ref = mixnet_client_guard.as_ref()?;
        let mixnet_sender = mixnet_client_ref.split_sender();
        let nym_address = *mixnet_client_ref.nym_address();

        Some(
            AuthenticatorMixnetClient::new(
                mixnet_sender,
                self.message_broadcast.subscribe(),
                nym_address,
            )
            .await,
        )
    }

    pub fn subscribe(&self) -> MixnetMessageBroadcastReceiver {
        self.message_broadcast.subscribe()
    }

    pub async fn wait(self) {
        if let Err(err) = self.handle.await {
            tracing::error!("Error waiting for auth clients mixnet listener to stop: {err}");
        }
    }
}
