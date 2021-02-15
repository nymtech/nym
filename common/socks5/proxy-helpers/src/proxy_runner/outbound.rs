// Copyright 2021 Nym Technologies SA
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

use super::SHUTDOWN_TIMEOUT;
use crate::connection_controller::{ConnectionMessage, ConnectionReceiver};
use futures::FutureExt;
use log::*;
use socks5_requests::ConnectionId;
use std::{sync::Arc, time::Duration};
use tokio::prelude::*;
use tokio::select;
use tokio::stream::StreamExt;
use tokio::{
    net::tcp::OwnedWriteHalf,
    sync::Notify,
    time::{delay_for, Instant},
};

const MIX_TTL: Duration = Duration::from_secs(5 * 60);

async fn deal_with_message(
    connection_message: ConnectionMessage,
    writer: &mut OwnedWriteHalf,
    local_destination_address: &str,
    remote_source_address: &str,
    connection_id: ConnectionId,
) -> bool {
    debug!(
        target: &*format!("({}) socks5 outbound", connection_id),
        "[{} bytes]\t{} → remote → mixnet → local → {} Remote closed: {}",
        connection_message.payload.len(),
        remote_source_address,
        local_destination_address,
        connection_message.socket_closed
    );

    if let Err(err) = writer.write_all(&connection_message.payload).await {
        // the other half is probably going to blow up too (if not, this task also needs to notify the other one!!)
        error!(target: &*format!("({}) socks5 outbound", connection_id), "failed to write response back to the socket - {}", err);
        return true;
    }
    if connection_message.socket_closed {
        debug!(target: &*format!("({}) socks5 outbound", connection_id),
               "Remote socket got closed - closing the local socket too");
        return true;
    }
    false
}

pub(super) async fn run_outbound(
    mut writer: OwnedWriteHalf,
    local_destination_address: String, // addresses are provided for better logging
    remote_source_address: String,
    mut mix_receiver: ConnectionReceiver,
    connection_id: ConnectionId,
    shutdown_notify: Arc<Notify>,
) -> (OwnedWriteHalf, ConnectionReceiver) {
    let shutdown_future = shutdown_notify
        .notified()
        .then(|_| delay_for(SHUTDOWN_TIMEOUT));

    tokio::pin!(shutdown_future);

    let mut mix_timeout = delay_for(MIX_TTL);

    loop {
        select! {
            connection_message = &mut mix_receiver.next() => {
                if let Some(connection_message) = connection_message {
                    if deal_with_message(connection_message, &mut writer, &local_destination_address, &remote_source_address, connection_id).await {
                        break;
                    }
                    mix_timeout.reset(Instant::now() + MIX_TTL);
                } else {
                    warn!("mix receiver is none so we already got removed somewhere. This isn't really a warning, but shouldn't happen to begin with, so please say if you see this message");
                    break;
                }
            }
            _ = &mut mix_timeout => {
                warn!("didn't get anything from the client on {} mixnet in {:?}. Shutting down the proxy.", connection_id, MIX_TTL);
                // If they were online it's kinda their fault they didn't send any heartbeat messages.
                break;
            }
            _ = &mut shutdown_future => {
                debug!("closing outbound proxy after inbound was closed {:?} ago", SHUTDOWN_TIMEOUT);
                break;
            }
        }
    }

    trace!("{} - outbound closed", connection_id);
    shutdown_notify.notify();

    (writer, mix_receiver)
}
