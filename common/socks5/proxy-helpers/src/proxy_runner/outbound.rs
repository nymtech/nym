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

use crate::connection_controller::ConnectionReceiver;
use log::*;
use socks5_requests::ConnectionId;
use tokio::net::tcp::{OwnedWriteHalf};
use tokio::prelude::*;
use tokio::stream::StreamExt;

pub(super) async fn run_outbound(
    mut writer: OwnedWriteHalf,
    local_destination_address: String, // addresses are provided for better logging
    remote_source_address: String,
    mut mix_receiver: ConnectionReceiver,
    connection_id: ConnectionId,
) -> (OwnedWriteHalf, ConnectionReceiver) {
    loop {
        let mix_data = mix_receiver.next().await;
        if mix_data.is_none() {
            warn!("mix receiver is none so we already got removed somewhere. This isn't really a warning, but shouldn't happen to begin with, so please say if you see this message");
            break;
        }
        let connection_message = mix_data.unwrap();

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
            break;
        }
        if connection_message.socket_closed {
            debug!(target: &*format!("({}) socks5 outbound", connection_id),
                  "Remote socket got closed - closing the local socket too");
            break;
        }
    }

    (writer, mix_receiver)
}