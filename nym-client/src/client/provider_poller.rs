use crate::client::FETCH_MESSAGES_DELAY;
use crate::utils;
use futures::channel::mpsc;
use log::{debug, error, info, trace, warn};
use provider_client::ProviderClientError;
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use std::net::SocketAddr;
use std::time::Duration;

pub(crate) struct ProviderPoller {
    provider_client: provider_client::ProviderClient,
    poller_tx: mpsc::UnboundedSender<Vec<Vec<u8>>>,
}

impl ProviderPoller {
    pub(crate) fn new(
        poller_tx: mpsc::UnboundedSender<Vec<Vec<u8>>>,
        provider_client_listener_address: SocketAddr,
        client_address: DestinationAddressBytes,
        auth_token: Option<AuthToken>,
    ) -> Self {
        ProviderPoller {
            provider_client: provider_client::ProviderClient::new(
                provider_client_listener_address,
                client_address,
                auth_token,
            ),
            poller_tx,
        }
    }

    // This method is only temporary until registration is moved to `client init`
    pub(crate) async fn perform_initial_registration(&mut self) -> Result<(), ProviderClientError> {
        debug!("performing initial provider registration");

        if !self.provider_client.is_registered() {
            let auth_token = match self.provider_client.register().await {
                // in this particular case we can ignore this error
                Err(ProviderClientError::ClientAlreadyRegisteredError) => return Ok(()),
                Err(err) => return Err(err),
                Ok(token) => token,
            };

            self.provider_client.update_token(auth_token)
        } else {
            warn!("did not perform registration - we were already registered")
        }

        Ok(())
    }

    pub(crate) async fn start_provider_polling(mut self) {
        info!("Starting provider poller");

        let loop_message = &utils::sphinx::LOOP_COVER_MESSAGE_PAYLOAD.to_vec();
        let dummy_message = &sfw_provider_requests::DUMMY_MESSAGE_CONTENT.to_vec();

        let delay_duration = Duration::from_secs_f64(FETCH_MESSAGES_DELAY);
        let extended_delay_duration = Duration::from_secs_f64(FETCH_MESSAGES_DELAY * 10.0);

        loop {
            debug!("Polling provider...");

            let messages = match self.provider_client.retrieve_messages().await {
                Err(err) => {
                    error!("Failed to query the provider for messages... Going to wait {:?} before retrying", extended_delay_duration);
                    tokio::time::delay_for(extended_delay_duration).await;
                    continue;
                }
                Ok(messages) => messages,
            };

            let good_messages = messages
                .into_iter()
                .filter(|message| message != loop_message && message != dummy_message)
                .collect();
            trace!("Obtained the following messages: {:?}", good_messages);

            // if this one fails, there's no retrying because it means that either:
            // - we run out of memory
            // - the receiver channel is closed
            // in either case there's no recovery and we can only panic
            self.poller_tx.unbounded_send(good_messages).unwrap();

            tokio::time::delay_for(delay_duration).await;
        }
    }
}
