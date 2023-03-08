// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::MixProxySender;
use super::SHUTDOWN_TIMEOUT;
use crate::available_reader::AvailableReader;
use bytes::Bytes;
use futures::FutureExt;
use futures::StreamExt;
use log::*;
use nym_ordered_buffer::OrderedMessageSender;
use nym_socks5_requests::ConnectionId;
use nym_task::connections::LaneQueueLengths;
use nym_task::connections::TransmissionLane;
use nym_task::TaskClient;
use std::fmt::Debug;
use std::time::Duration;
use std::{io, sync::Arc};
use tokio::select;
use tokio::{net::tcp::OwnedReadHalf, sync::Notify, time::sleep};

async fn send_empty_close<F, S>(
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
) where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S,
    S: Debug,
{
    let ordered_msg = message_sender.wrap_message(Vec::new()).into_bytes();
    mix_sender
        .send(adapter_fn(connection_id, ordered_msg, true))
        .await
        .expect("BatchRealMessageReceiver has stopped receiving!");
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
    S: Debug,
{
    let (read_data, is_finished) = match read_data {
        Some(data) => match data {
            Ok(data) => (data, false),
            Err(err) => {
                error!(target: &*format!("({connection_id}) socks5 inbound"), "failed to read request from the socket - {err}");
                (Default::default(), true)
            }
        },
        None => (Default::default(), true),
    };

    debug!(
        target: &*format!("({connection_id}) socks5 inbound"),
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

    // Before sending the data downstream, wait for the lane at the `OutQueueControl` is reasonably
    // close to finishing. This is a way of pacing the sending application (backpressure).
    if let Some(ref lane_queue_lengths) = lane_queue_lengths {
        // We allow a bit of slack to try to keep the pipeline >0
        wait_until_lane_almost_empty(lane_queue_lengths, connection_id).await;
    }

    mix_sender
        .send(adapter_fn(connection_id, ordered_msg, is_finished))
        .await
        .expect("InputMessageReceiver has stopped receiving!");

    if is_finished {
        // After sending, if this is the last message, wait until we've actually transmitted the data
        // in the `OutQueueControl` and the lane is empty.
        if let Some(ref lane_queue_lengths) = lane_queue_lengths {
            // This is basically an ugly workaround to make sure that we don't start waiting until
            // the data that we pushed arrived at the OutQueueControl.
            // This usually not a problem in the socks5-client, but for the network-requester this
            // info is synced at up to every 500ms.
            sleep(Duration::from_secs(2)).await;
            wait_until_lane_empty(lane_queue_lengths, connection_id).await;
        }

        // Technically we already informed it when we sent the message to mixnet above
        debug!(
            target: &*format!("({connection_id}) socks5 inbound"),
            "The local socket is closed - won't receive any more data. Informing remote about that..."
        );
    }

    is_finished
}

async fn wait_until_lane_empty(lane_queue_lengths: &LaneQueueLengths, connection_id: u64) {
    if tokio::time::timeout(
        Duration::from_secs(4 * 60),
        wait_for_lane(
            lane_queue_lengths,
            connection_id,
            0,
            Duration::from_millis(500),
        ),
    )
    .await
    .is_err()
    {
        log::warn!("Wait until lane empty timed out");
    }
}

async fn wait_until_lane_almost_empty(lane_queue_lengths: &LaneQueueLengths, connection_id: u64) {
    if tokio::time::timeout(
        Duration::from_secs(4 * 60),
        wait_for_lane(
            lane_queue_lengths,
            connection_id,
            30,
            Duration::from_millis(100),
        ),
    )
    .await
    .is_err()
    {
        log::debug!("Wait until lane almost empty timed out");
    }
}

async fn wait_for_lane(
    lane_queue_lengths: &LaneQueueLengths,
    connection_id: u64,
    queue_length_threshold: usize,
    sleep_duration: Duration,
) {
    while let Some(queue) = lane_queue_lengths.get(&TransmissionLane::ConnectionId(connection_id)) {
        if queue > queue_length_threshold {
            sleep(sleep_duration).await;
        } else {
            break;
        }
    }
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
    mut shutdown_listener: TaskClient,
) -> OwnedReadHalf
where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
    S: Debug,
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

    shutdown_listener.mark_as_success();
    reader
}
