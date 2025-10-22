// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use nym_common::trace_err_chain;
use nym_sdk::mixnet::{
    InputMessage, MixnetClientSender, MixnetMessageSender, Recipient, TransmissionLane,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

use crate::{Error, error::Result, nym_ip_packet_requests_current::request::IpPacketRequest};

const MIXNET_SELF_PING_INTERVAL: Duration = Duration::from_millis(1000);

struct MixnetConnectionBeacon {
    mixnet_client_sender: MixnetClientSender,
    our_address: Recipient,
}

impl MixnetConnectionBeacon {
    fn new(mixnet_client_sender: MixnetClientSender, our_address: Recipient) -> Self {
        MixnetConnectionBeacon {
            mixnet_client_sender,
            our_address,
        }
    }

    async fn send_mixnet_self_ping(&self) -> Result<u64> {
        trace!("Sending mixnet self ping");
        let (input_message, request_id) = create_self_ping(self.our_address);
        self.mixnet_client_sender
            .send(input_message)
            .await
            .map_err(|err| Error::NymSdkError(Box::new(err)))?;
        Ok(request_id)
    }

    pub async fn run(self, shutdown: CancellationToken) -> Result<()> {
        debug!("Mixnet connection beacon is running");
        let mut ping_interval = tokio::time::interval(MIXNET_SELF_PING_INTERVAL);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    trace!("MixnetConnectionBeacon: Received shutdown");
                    break;
                }
                _ = ping_interval.tick() => {
                    tokio::select! {
                        _ = shutdown.cancelled() => {
                            trace!("MixnetConnectionBeacon: Received shutdown");
                            break;
                        },
                        ping_result = self.send_mixnet_self_ping() => {
                            let _ping_id = match ping_result {
                                Ok(id) => id,
                                Err(err) => {
                                    trace_err_chain!(
                                        err,
                                        "Failed to send mixnet self ping"
                                    );
                                    continue;
                                }
                            };
                            // TODO: store ping_id to be able to monitor or ping timeouts
                        }
                    };
                }
            }
        }
        debug!("MixnetConnectionBeacon: Exiting");
        Ok(())
    }
}

pub fn create_self_ping(our_address: Recipient) -> (InputMessage, u64) {
    let (request, request_id) = IpPacketRequest::new_ping();
    (
        InputMessage::new_regular(
            our_address,
            // SAFETY: this message has infallible serialisation
            #[allow(clippy::unwrap_used)]
            request.to_bytes().unwrap(),
            TransmissionLane::General,
            None,
            None,
        ),
        request_id,
    )
}

pub fn start_mixnet_connection_beacon(
    mixnet_client_sender: MixnetClientSender,
    our_address: Recipient,
    cancel_token: CancellationToken,
) -> JoinHandle<Result<()>> {
    debug!("Creating mixnet connection beacon");
    let beacon = MixnetConnectionBeacon::new(mixnet_client_sender, our_address);
    tokio::spawn(async move {
        beacon
            .run(cancel_token)
            .await
            .inspect_err(|err| trace_err_chain!(err, "Mixnet connection beacon error"))
    })
}
