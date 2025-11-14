// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, Recipient};
use tracing::{debug, error};

use crate::{
    error::{Error, Result},
    mixnet_beacon::create_self_ping,
    nym_ip_packet_requests_current::request::IpPacketRequest,
};

// Send mixnet self ping and wait for the response
pub async fn self_ping_and_wait(
    our_address: Recipient,
    mixnet_client: &mut MixnetClient,
) -> Result<()> {
    let request_ids = send_self_pings(our_address, mixnet_client).await?;
    wait_for_self_ping_return(mixnet_client, &request_ids).await
}

async fn send_self_pings(
    our_address: Recipient,
    mixnet_client: &mut MixnetClient,
) -> Result<Vec<u64>> {
    let mut request_ids = Vec::with_capacity(3);

    for _ in 1..=3 {
        let (input_message, request_id) = create_self_ping(our_address);
        mixnet_client
            .send(input_message)
            .await
            .map_err(|err| Error::NymSdkError(Box::new(err)))?;
        request_ids.push(request_id);
    }

    Ok(request_ids)
}

async fn wait_for_self_ping_return(
    mixnet_client: &mut MixnetClient,
    request_ids: &[u64],
) -> Result<()> {
    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                error!("Timed out waiting for mixnet self ping to return");
                return Err(Error::TimeoutWaitingForMixnetSelfPing);
            }
            Some(msgs) = mixnet_client.wait_for_messages() => {
                for msg in msgs {
                    let Ok(response) = IpPacketRequest::from_reconstructed_message(&msg) else {
                        // This is a common case when we are reconnecting to a gateway and receive
                        // all sorts of messages that are buffered since out last connection.
                        debug!("Failed to deserialize reconstructed message");
                        continue;
                    };
                    if request_ids.iter().any(|&id| response.id() == Some(id)) {
                        debug!("Got the ping we were waiting for");
                        return Ok(());
                    }
                }
            }
        }
    }
}
