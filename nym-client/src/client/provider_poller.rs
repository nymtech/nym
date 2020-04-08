// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::channel::mpsc;
use log::*;
use provider_client::ProviderClientError;
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use std::net::SocketAddr;
use std::time;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub(crate) type PolledMessagesSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type PolledMessagesReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

pub(crate) struct ProviderPoller {
    polling_rate: time::Duration,
    provider_client: provider_client::ProviderClient,
    poller_tx: mpsc::UnboundedSender<Vec<Vec<u8>>>,
}

impl ProviderPoller {
    pub(crate) fn new(
        poller_tx: mpsc::UnboundedSender<Vec<Vec<u8>>>,
        provider_client_listener_address: SocketAddr,
        client_address: DestinationAddressBytes,
        auth_token: Option<AuthToken>,
        polling_rate: time::Duration,
    ) -> Self {
        ProviderPoller {
            provider_client: provider_client::ProviderClient::new(
                provider_client_listener_address,
                client_address,
                auth_token,
            ),
            poller_tx,
            polling_rate,
        }
    }

    pub(crate) fn is_registered(&self) -> bool {
        self.provider_client.is_registered()
    }

    // This method is only temporary until registration is moved to `client init`
    pub(crate) async fn perform_initial_registration(&mut self) -> Result<(), ProviderClientError> {
        debug!("performing initial provider registration");

        if !self.is_registered() {
            let auth_token = match self.provider_client.register().await {
                // in this particular case we can ignore this error
                Err(ProviderClientError::ClientAlreadyRegisteredError) => return Ok(()),
                Err(err) => return Err(err),
                Ok(token) => token,
            };

            self.provider_client.update_token(auth_token)
        } else {
            warn!("did not perform provider registration - we were already registered")
        }

        Ok(())
    }

    pub(crate) async fn start_provider_polling(self) {
        let loop_message = &mix_client::packet::LOOP_COVER_MESSAGE_PAYLOAD.to_vec();
        let dummy_message = &sfw_provider_requests::DUMMY_MESSAGE_CONTENT.to_vec();

        let extended_delay_duration = self.polling_rate * 10;

        loop {
            debug!("Polling provider...");

            let messages = match self.provider_client.retrieve_messages().await {
                Err(err) => {
                    error!("Failed to query the provider for messages: {:?}, ... Going to wait {:?} before retrying", err, extended_delay_duration);
                    tokio::time::delay_for(extended_delay_duration).await;
                    continue;
                }
                Ok(messages) => messages,
            };

            // todo: this will probably need to be updated at some point due to changed structure
            // of nym-sphinx messages sent. However, for time being this is still compatible.
            // Basically it is more of a personal note of if client keeps getting errors and is
            // not getting messages, look at this filter here.
            let good_messages: Vec<_> = messages
                .into_iter()
                .filter(|message| message != loop_message && message != dummy_message)
                .collect();
            trace!("Obtained the following messages: {:?}", good_messages);

            // if this one fails, there's no retrying because it means that either:
            // - we run out of memory
            // - the receiver channel is closed
            // in either case there's no recovery and we can only panic
            if !good_messages.is_empty() {
                self.poller_tx.unbounded_send(good_messages).unwrap();
            }

            tokio::time::delay_for(self.polling_rate).await;
        }
    }

    pub(crate) fn start(self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move { self.start_provider_polling().await })
    }
}
