// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::MixProxySender;
use super::SHUTDOWN_TIMEOUT;
use crate::available_reader::AvailableReader;
use bytes::Bytes;
use client_connections::LaneQueueLengths;
use client_connections::TransmissionLane;
use futures::FutureExt;
use futures::StreamExt;
use log::*;
use ordered_buffer::OrderedMessageSender;
use socks5_requests::ConnectionId;
use std::time::Duration;
use std::{io, sync::Arc};
use task::ShutdownListener;
use tokio::select;
use tokio::time;
use tokio::{net::tcp::OwnedReadHalf, sync::Notify, time::sleep};

async fn send_empty_close<F, S>(
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
) where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S,
{
    let ordered_msg = message_sender.wrap_message(Vec::new()).into_bytes();
    if mix_sender
        .send(adapter_fn(connection_id, ordered_msg, true))
        .await
        .is_err()
    {
        panic!();
    }
}

#[allow(clippy::too_many_arguments)]
async fn deal_with_data<F, S>(
    read_data: Option<io::Result<Bytes>>,
    local_destination_address: &str,
    remote_source_address: &str,
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
    lane_queue_lengths: Option<LaneQueueLengths>,
) -> bool
where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S,
{
    let (read_data, is_finished) = match read_data {
        Some(data) => match data {
            Ok(data) => (data, false),
            Err(err) => {
                error!(target: &*format!("({}) socks5 inbound", connection_id), "failed to read request from the socket - {}", err);
                (Default::default(), true)
            }
        },
        None => (Default::default(), true),
    };

    debug!(
        target: &*format!("({}) socks5 inbound", connection_id),
        "[{} bytes]\t{} → local → mixnet → remote → {}. Local closed: {}",
        read_data.len(),
        local_destination_address,
        remote_source_address,
        is_finished
    );

    // if we're sending through the mixnet increase the sequence number...
    let ordered_msg = message_sender.wrap_message(read_data.to_vec()).into_bytes();
    log::trace!(
        "pushing data down the input sender: size: {}",
        ordered_msg.len()
    );

    // If we are closing the channel, wait until the data has passed `OutQueueControl` and the lane
    // is empty.
    if let Some(lane_queue_lengths) = lane_queue_lengths {
        if is_finished {
            while let Some(queue) =
                lane_queue_lengths.get(&TransmissionLane::ConnectionId(connection_id))
            {
                if queue > 0 {
                    sleep(Duration::from_millis(500)).await;
                } else {
                    break;
                }
            }
        }
    }

    if mix_sender
        .send(adapter_fn(connection_id, ordered_msg, is_finished))
        .await
        .is_err()
    {
        panic!();
    }

    // If we receive large data sets then what might happen is that we push all the data down the
    // channel and the close it before the lane queue lenghts are updated in `OutQueueControl`. So
    // we back off ever so slightly here to give some time for them to reach the `OutQueueControl`
    // and the lane queue lengths being updated until the next message for this connection.

    // TODO: this is all hardcoded and ugly. The correct solution I hope would be to rework
    // `OutQueueControl` to poll it's incoming channel independently on the poisson delay.
    if read_data.len() > 50_000 {
        // Heuristic used:
        // average delay: 20ms => 50 packets/sec
        // ~1.5 kB/packet => ~75 kB/s
        let time_to_send = read_data.len() / 75;
        // Dont need to wait until we've sent the data, just something that is proportional to it.
        let fraction_of_time_to_send = time_to_send / 10;
        time::sleep(Duration::from_millis(fraction_of_time_to_send as u64)).await;
    }

    if is_finished {
        // technically we already informed it when we sent the message to mixnet above
        debug!(target: &*format!("({}) socks5 inbound", connection_id), "The local socket is closed - won't receive any more data. Informing remote about that...");
    }

    is_finished
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_inbound<F, S>(
    mut reader: OwnedReadHalf,
    local_destination_address: String, // addresses are provided for better logging
    remote_source_address: String,
    connection_id: ConnectionId,
    mix_sender: MixProxySender<S>,
    adapter_fn: F,
    shutdown_notify: Arc<Notify>,
    lane_queue_lengths: Option<LaneQueueLengths>,
    mut shutdown_listener: ShutdownListener,
) -> OwnedReadHalf
where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
{
    let mut available_reader = AvailableReader::new(&mut reader);
    let mut message_sender = OrderedMessageSender::new();
    let shutdown_future = shutdown_notify.notified().then(|_| sleep(SHUTDOWN_TIMEOUT));

    tokio::pin!(shutdown_future);

    loop {
        select! {
            read_data = &mut available_reader.next() => {
                if deal_with_data(
                    read_data,
                    &local_destination_address,
                    &remote_source_address,
                    connection_id,
                    &mut message_sender,
                    &mix_sender,
                    &adapter_fn,
                    lane_queue_lengths.clone()
                ).await {
                    break
                }
            }
            _ = &mut shutdown_future => {
                debug!(
                    "closing inbound proxy after outbound was closed {:?} ago",
                    SHUTDOWN_TIMEOUT
                );
                // inform remote just in case it was closed because of lack of heartbeat.
                // worst case the remote will just have couple of false negatives
                send_empty_close(connection_id, &mut message_sender, &mix_sender, &adapter_fn).await;
                break;
            }
            _ = shutdown_listener.recv() => {
                log::trace!("ProxyRunner inbound: Received shutdown");
                break;
            }
        }
    }
    trace!("{} - inbound closed", connection_id);
    shutdown_notify.notify_one();

    reader
}
