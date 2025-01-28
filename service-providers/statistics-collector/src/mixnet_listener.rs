// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{error::*, storage::ClientStatsStorage};
use futures::StreamExt;
use nym_sphinx::receiver::ReconstructedMessage;
use nym_statistics_common::report::ClientStatsReport;
use nym_task::TaskHandle;

pub(crate) struct MixnetListener {
    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // Report storage
    pub(crate) client_report_storage: ClientStatsStorage,

    // The task handle for the main loop
    pub(crate) task_handle: TaskHandle,
}

impl MixnetListener {
    pub fn new(
        mixnet_client: nym_sdk::mixnet::MixnetClient,
        client_report_storage: ClientStatsStorage,
        task_handle: TaskHandle,
    ) -> Self {
        MixnetListener {
            mixnet_client,
            client_report_storage,
            task_handle,
        }
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> Result<String> {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );
        let report = deserialize_stats_report(&reconstructed)?;
        self.client_report_storage
            .store_report(report.clone())
            .await?;
        Ok(report.client_id)
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let mut task_client = self.task_handle.fork("main_loop");

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("Statistics collector [main loop]: received shutdown");
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(client_id) => {
                                log::error!("Successfully stored client reports from ID : {client_id}")
                            },
                            Err(err) => {
                                log::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        log::trace!("Statistics collector [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        log::debug!("Statistics collector: stopping");
        Ok(())
    }
}

fn deserialize_stats_report(reconstructed: &ReconstructedMessage) -> Result<ClientStatsReport> {
    let report_bytes: &[u8] = &reconstructed.message;
    Ok(report_bytes.try_into()?)
}
