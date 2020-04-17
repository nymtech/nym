// Copyright 2020 Nym Technologies SA
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

use super::mixnet_client;
use futures::lock::Mutex;
use futures_util::StreamExt;
use log::*;
use multi_tcp_client::Client as MultiClient;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tungstenite::Result;

pub async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    client_ref: Arc<Mutex<MultiClient>>,
) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    info!("New WebSocket connection: {}", peer);

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_binary() {
            mixnet_client::forward_to_mixnode(msg.into_data(), Arc::clone(&client_ref)).await;
        }
    }
    Ok(())
}
