// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::MixProxySender;
use super::SHUTDOWN_TIMEOUT;
use crate::available_reader::AvailableReader;
use bytes::Bytes;
use client_connections::LaneQueueLength;
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
use tokio::{net::tcp::OwnedReadHalf, sync::Notify, time::sleep};

fn send_empty_close<F, S>(
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
) where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S,
{
    let ordered_msg = message_sender.wrap_message(Vec::new()).into_bytes();
    mix_sender
        .unbounded_send(adapter_fn(connection_id, ordered_msg, true))
        .unwrap();
}

async fn deal_with_data<F, S>(
    read_data: Option<io::Result<Bytes>>,
    local_destination_address: &str,
    remote_source_address: &str,
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
    lane_queue_length: LaneQueueLength,
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

    // wait here until queue is not too long
    let lane = TransmissionLane::ConnectionId(connection_id);
    //loop {
    {
        let (queue_length, est_busy_conn) = {
            let mut guard = lane_queue_length.lock().unwrap();
            //let queue_length = *guard.get(&lane).unwrap_or(&0);
            //let queue_length = *guard.entry(lane).or_insert(0);
            let queue_length = guard.get(&lane).unwrap_or(0);
            let est_busy_conn = guard.values().filter(|v| *v > &5).count();

            // We estimate the queue length for subsequent data packet. This is needed
            // because there is a delay until we get the correct value back as a server
            // response. (And that will have a delay baked in).
            // WIP(JON): pull packet size from somewhere
            let sphinx_size = 2000.0;
            let msg_length = (ordered_msg.len() as f64 / sphinx_size).ceil() as usize;
            guard.modify(&lane, |length| *length += msg_length);

            (queue_length, est_busy_conn)
        };

        log::info!("conn_id: {connection_id}, queue: {queue_length}");
        // The heuristic here is:
        // 20ms average delay => 50 packets / sec
        // 500 packet queue => 10 sec behind
        // This assumes it's the only active connection, and that there is no throttling
        // In practive, this is a latency vs throughput tradeoff we're making here
        let avererage_delay = 0.02; // TODO: read from config
        let packets_per_sec = 1.0 / avererage_delay;
        let ideal_time_to_clear_queue = queue_length as f64 / packets_per_sec;
        if queue_length > 5000 {
            log::info!("sleeping long");
            sleep(Duration::from_secs_f64(ideal_time_to_clear_queue * 5.0)).await;
        } else if queue_length > 500 {
            log::info!("sleeping medium");
            sleep(Duration::from_secs_f64(
                ideal_time_to_clear_queue * 2.0 / 3.0,
            ))
            .await;
        } else if queue_length > 5 {
            log::info!("sleeping short");
            sleep(Duration::from_secs_f64(ideal_time_to_clear_queue / 3.0)).await;
        }

        // If we are saturated on number of connections, and this is already a busy
        // connection, basically soft-stop
        //if est_busy_conn > 15 && queue_length > 5 {
        //    log::info!("soft-stop: {connection_id}");
        //    sleep(Duration::from_secs_f64(5.0 * 60.0)).await;
        //}

        //loop {
        //    let count = ACTIVE_PROXIES.load(Ordering::Relaxed);
        //    if count + 1 > 15 {
        //        log::info!("Max connections reached, parking: {conn_id}");
        //        sleep(Duration::from_secs(10)).await;
        //    } else {
        //        break;
        //    }
        //}
    }

    mix_sender
        .unbounded_send(adapter_fn(connection_id, ordered_msg, is_finished))
        .unwrap();

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
    lane_queue_length: LaneQueueLength,
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
                    lane_queue_length.clone()
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
                send_empty_close(connection_id, &mut message_sender, &mix_sender, &adapter_fn);
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
