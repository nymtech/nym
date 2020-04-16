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

use futures_util::{SinkExt, StreamExt};
use log::*;
use std::sync::{Arc};
use futures::lock::Mutex;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error};
use tungstenite::Message;
use tungstenite::Result;

async fn accept_connection(peer: SocketAddr, stream: TcpStream, client: Arc<Mutex<multi_tcp_client::Client>>) {
    
    if let Err(e) = handle_connection(peer, stream, client).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream, client: Arc<Mutex<multi_tcp_client::Client>>) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    info!("New WebSocket connection: {}", peer);

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_text() {
            info!("Got text message: {}", msg);
            let response = Message::Text("Text on this socket is ignored".to_owned());
            ws_stream.send(response).await?;
        }
        if msg.is_binary() {
            info!("Got binary message: {}", msg);
            let address: SocketAddr = "127.0.0.1:9980".parse().unwrap();
            let mut foomp = client.lock().await;
            foomp.send(address, msg.into_data(), false).await.unwrap();
        }
    }

    Ok(())
}



#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    setup_logging();
    let addr = "127.0.0.1:1793";
    let mut listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    let client = setup_client();

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(peer, stream, client.clone()));
    }
}

fn setup_client() -> Arc<Mutex<multi_tcp_client::Client>> {
        let config = multi_tcp_client::Config::new(
        Duration::from_millis(200),
        Duration::from_secs(86400),
        Duration::from_secs(2),
    );
    let client = multi_tcp_client::Client::new(config);
    Arc::new(Mutex::new(client))
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .init();
}
