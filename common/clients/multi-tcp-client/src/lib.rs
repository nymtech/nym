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

use crate::connection_manager::{ConnectionManager, ConnectionManagerSender};
use futures::channel::oneshot;
use futures::future::AbortHandle;
use log::*;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;

mod connection_manager;

pub struct Config {
    initial_reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
        }
    }
}

pub struct Client {
    runtime_handle: Handle,
    connections_managers: HashMap<SocketAddr, (ConnectionManagerSender, AbortHandle)>,
    maximum_reconnection_backoff: Duration,
    initial_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            // if the function is not called within tokio runtime context, this will panic
            // but perhaps the code should be better structured to completely avoid this call
            runtime_handle: Handle::try_current()
                .expect("The client MUST BE used within tokio runtime context"),
            connections_managers: HashMap::new(),
            initial_reconnection_backoff: config.maximum_reconnection_backoff,
            maximum_reconnection_backoff: config.initial_reconnection_backoff,
            initial_connection_timeout: config.initial_connection_timeout,
        }
    }

    async fn start_new_connection_manager(
        &mut self,
        address: SocketAddr,
    ) -> (ConnectionManagerSender, AbortHandle) {
        let (sender, abort_handle) = ConnectionManager::new(
            address,
            self.initial_reconnection_backoff,
            self.maximum_reconnection_backoff,
            self.initial_connection_timeout,
        )
        .await
        .start_abortable(&self.runtime_handle);

        (sender, abort_handle)
    }

    // if wait_for_response is set to true, we will get information about any possible IO errors
    // as well as (once implemented) received replies, however, this will also cause way longer
    // waiting periods
    pub async fn send(
        &mut self,
        address: SocketAddr,
        message: Vec<u8>,
        wait_for_response: bool,
    ) -> io::Result<()> {
        if !self.connections_managers.contains_key(&address) {
            debug!(
                "There is no existing connection to {:?} - it will be established now",
                address
            );

            let (new_manager_sender, abort_handle) =
                self.start_new_connection_manager(address).await;
            self.connections_managers
                .insert(address, (new_manager_sender, abort_handle));
        }

        let manager = self.connections_managers.get_mut(&address).unwrap();

        if wait_for_response {
            let (res_tx, res_rx) = oneshot::channel();
            manager.0.unbounded_send((message, Some(res_tx))).unwrap();
            res_rx.await.unwrap()
        } else {
            manager.0.unbounded_send((message, None)).unwrap();
            Ok(())
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        for (_, abort_handle) in self.connections_managers.values() {
            abort_handle.abort()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use std::time;
    use tokio::prelude::*;

    const SERVER_MSG_LEN: usize = 16;
    const CLOSE_MESSAGE: [u8; SERVER_MSG_LEN] = [0; SERVER_MSG_LEN];

    struct DummyServer {
        received_buf: Vec<Vec<u8>>,
        listener: tokio::net::TcpListener,
    }

    impl DummyServer {
        async fn new(address: SocketAddr) -> Self {
            DummyServer {
                received_buf: Vec::new(),
                listener: tokio::net::TcpListener::bind(address).await.unwrap(),
            }
        }

        fn get_received(&self) -> Vec<Vec<u8>> {
            self.received_buf.clone()
        }

        // this is only used in tests so slightly higher logging levels are fine
        async fn listen_until(mut self, close_message: &[u8]) -> Self {
            let (mut socket, _) = self.listener.accept().await.unwrap();
            loop {
                let mut buf = [0u8; SERVER_MSG_LEN];
                match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => {
                        info!("Remote connection closed");
                        return self;
                    }
                    Ok(n) => {
                        info!("received ({}) - {:?}", n, str::from_utf8(buf[..n].as_ref()));

                        if buf[..n].as_ref() == close_message {
                            info!("closing...");
                            socket.shutdown(std::net::Shutdown::Both).unwrap();
                            return self;
                        } else {
                            self.received_buf.push(buf[..n].to_vec());
                        }
                    }
                    Err(e) => {
                        panic!("failed to read from socket; err = {:?}", e);
                    }
                };
            }
        }
    }

    #[test]
    fn client_reconnects_to_server_after_it_went_down() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let addr = "127.0.0.1:6000".parse().unwrap();
        let reconnection_backoff = Duration::from_secs(1);
        let timeout = Duration::from_secs(1);
        let client_config = Config::new(reconnection_backoff, 10 * reconnection_backoff, timeout);

        let messages_to_send = vec![[1u8; SERVER_MSG_LEN].to_vec(), [2; SERVER_MSG_LEN].to_vec()];

        let dummy_server = rt.block_on(DummyServer::new(addr));
        let finished_dummy_server_future = rt.spawn(dummy_server.listen_until(&CLOSE_MESSAGE));

        let mut c = rt.enter(|| Client::new(client_config));

        for msg in &messages_to_send {
            rt.block_on(c.send(addr, msg.clone(), true)).unwrap();
        }

        // kill server
        rt.block_on(c.send(addr, CLOSE_MESSAGE.to_vec(), true))
            .unwrap();
        let received_messages = rt
            .block_on(finished_dummy_server_future)
            .unwrap()
            .get_received();

        assert_eq!(received_messages, messages_to_send);

        // try to send - go into reconnection
        let post_kill_message = [3u8; SERVER_MSG_LEN].to_vec();

        // we are trying to send to killed server
        assert!(rt
            .block_on(c.send(addr, post_kill_message.clone(), true))
            .is_err());

        let new_dummy_server = rt.block_on(DummyServer::new(addr));
        let new_server_future = rt.spawn(new_dummy_server.listen_until(&CLOSE_MESSAGE));

        // keep sending after we leave reconnection backoff and reconnect
        loop {
            if rt
                .block_on(c.send(addr, post_kill_message.clone(), true))
                .is_ok()
            {
                break;
            }
            rt.block_on(
                async move { tokio::time::delay_for(time::Duration::from_millis(50)).await },
            );
        }

        // kill the server to ensure it actually got the message
        rt.block_on(c.send(addr, CLOSE_MESSAGE.to_vec(), true))
            .unwrap();
        let new_received_messages = rt.block_on(new_server_future).unwrap().get_received();
        assert_eq!(post_kill_message.to_vec(), new_received_messages[0]);
    }

    #[test]
    fn server_receives_all_sent_messages_when_up() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let addr = "127.0.0.1:6001".parse().unwrap();
        let reconnection_backoff = Duration::from_secs(2);
        let timeout = Duration::from_secs(1);
        let client_config = Config::new(reconnection_backoff, 10 * reconnection_backoff, timeout);

        let messages_to_send = vec![[1u8; SERVER_MSG_LEN].to_vec(), [2; SERVER_MSG_LEN].to_vec()];

        let dummy_server = rt.block_on(DummyServer::new(addr));
        let finished_dummy_server_future = rt.spawn(dummy_server.listen_until(&CLOSE_MESSAGE));

        let mut c = rt.enter(|| Client::new(client_config));

        for msg in &messages_to_send {
            rt.block_on(c.send(addr, msg.clone(), true)).unwrap();
        }

        rt.block_on(c.send(addr, CLOSE_MESSAGE.to_vec(), true))
            .unwrap();

        // the server future should have already been resolved
        let received_messages = rt
            .block_on(finished_dummy_server_future)
            .unwrap()
            .get_received();

        assert_eq!(received_messages, messages_to_send);
    }
}
