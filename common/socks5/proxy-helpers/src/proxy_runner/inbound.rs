// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::SHUTDOWN_TIMEOUT;
use crate::available_reader::AvailableReader;
use crate::ordered_sender::OrderedMessageSender;
use crate::proxy_runner::KEEPALIVE_INTERVAL;
use futures::FutureExt;
use futures::StreamExt;
use log::*;
use nym_socks5_requests::{ConnectionId, SocketData};
use nym_task::connections::LaneQueueLengths;
use nym_task::connections::TransmissionLane;
use nym_task::TaskClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::{net::tcp::OwnedReadHalf, sync::Notify, time::sleep};

async fn wait_until_lane_empty(lane_queue_lengths: &Option<LaneQueueLengths>, connection_id: u64) {
    if let Some(lane_queue_lengths) = lane_queue_lengths {
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
}

async fn wait_until_lane_almost_empty(
    lane_queue_lengths: &Option<LaneQueueLengths>,
    connection_id: u64,
) {
    if let Some(lane_queue_lengths) = lane_queue_lengths {
        if tokio::time::timeout(
            Duration::from_secs(4 * 60),
            wait_for_lane(
                lane_queue_lengths,
                connection_id,
                // With only 30 packets in the queue, we treat it as basically empty.
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

pub(super) async fn run_inbound<F, S: Send>(
    mut reader: OwnedReadHalf,
    mut message_sender: OrderedMessageSender<F, S>,
    connection_id: ConnectionId,
    available_plaintext_per_mix_packet: usize,
    shutdown_notify: Arc<Notify>,
    lane_queue_lengths: Option<LaneQueueLengths>,
    mut shutdown_listener: TaskClient,
) -> OwnedReadHalf
where
    F: Fn(SocketData) -> S + Send + 'static,
{
    // TODO: this multiplication by 4 is completely arbitrary here
    let mut available_reader =
        AvailableReader::new(&mut reader, Some(available_plaintext_per_mix_packet * 4));

    // Shutdown if outbound signled to shutdown
    let shutdown_future = shutdown_notify.notified().then(|_| sleep(SHUTDOWN_TIMEOUT));
    tokio::pin!(shutdown_future);

    // Timer to send empty keepalive messages
    let mut keepalive_timer = tokio::time::interval(KEEPALIVE_INTERVAL);

    // Once we finish read from the local socket, we need to wait until we've actually transmitted
    // until we can exit the task. Otherwise, if we close the connection while there is still
    // packets waiting to be sent in `OutQueueControl`, they will be dropped.
    let closing_notify = Arc::new(Notify::new());
    let closing_future = closing_notify
        .notified()
        .then(|_| {
            // We wait a little to make sure that the packets make their way to the
            // `OutQueueControl` and is registered in the `LaneQueueLengths`.
            //
            // NOTE: This is as hacky as it looks. My preferred approach would be to do the chunking in
            // each connection task and then send the chunks to the `OutQueueControl` directly.
            sleep(Duration::from_secs(2))
        })
        .then(|_| wait_until_lane_empty(&lane_queue_lengths, connection_id));
    tokio::pin!(closing_future);

    // Once we are closed, we need to disable the branch in the select that reads from the socket.
    let mut we_are_closed = false;

    loop {
        select! {
            biased;
            _ = &mut shutdown_future => {
                debug!(
                    "closing inbound proxy after outbound was closed {:?} ago",
                    SHUTDOWN_TIMEOUT
                );
                // inform remote just in case it was closed because of lack of heartbeat.
                // worst case the remote will just have couple of false negatives
                message_sender.send_empty_close().await;
                break;
            }
            _ = shutdown_listener.recv() => {
                log::trace!("ProxyRunner inbound: Received shutdown");
                break;
            }
            _ = &mut closing_future => {
                // Technically we already informed it when we sent the last message to mixnet
                debug!(
                    target: &*format!("({connection_id}) socks5 inbound"),
                    "The local socket is closed - won't receive any more data. Informing remote about that..."
                );
                break;
            }
            _ = keepalive_timer.tick() => {
                message_sender.send_empty_keepalive().await;
            }
            // Read the next data when there is space in the lane.
            // The purpose of chaining the wait here is that it makes sure we can cancel the
            // waiting on connection close.
            read_data = wait_until_lane_almost_empty(&lane_queue_lengths, connection_id)
                .then(|_| available_reader.next()), if !we_are_closed =>
            {
                let processed = message_sender.process_data(read_data);
                let is_done = processed.is_done;

                message_sender.send_data(processed).await;

                if is_done {
                    // After reading the last data, notify the closing_future to wait until the
                    // lane is clear before exiting.
                    // We don't wait here since we want to be able to cancel the wait on close or
                    // shutdown.
                    closing_notify.notify_one();
                    we_are_closed = true;
                }
                // No need to send keepalive messages when just sent real data
                keepalive_timer.reset();
            }
        }
    }
    trace!("{} - inbound closed", connection_id);
    shutdown_notify.notify_one();

    shutdown_listener.mark_as_success();
    reader
}
