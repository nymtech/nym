// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;

use crate::{config::Config, error::*};

pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) _config: Config,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // The task handle for the main loop
    pub(crate) task_handle: TaskHandle,
}

impl MixnetListener {
    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> Result<Vec<()>> {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );
        Ok(vec![])
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let mut task_client = self.task_handle.fork("main_loop");

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("Authenticator [main loop]: received shutdown");
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(_) => {},
                            Err(err) => {
                                log::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        log::trace!("Authenticator [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        log::debug!("Authenticator: stopping");
        Ok(())
    }
}
