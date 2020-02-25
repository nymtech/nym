use crate::connection_manager::ConnectionManager;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

mod connection_manager;

pub struct Config {
    initial_endpoints: Vec<SocketAddr>,
    reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
}

impl Config {
    pub fn new(
        initial_endpoints: Vec<SocketAddr>,
        reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
    ) -> Self {
        Config {
            initial_endpoints,
            reconnection_backoff,
            maximum_reconnection_backoff,
        }
    }
}

pub struct Client<'a> {
    connections_managers: HashMap<SocketAddr, ConnectionManager<'a>>,
}

impl<'a> Client<'a> {
    pub async fn new(config: Config) -> Client<'a> {
        let mut connections_managers = HashMap::new();
        for endpoint in config.initial_endpoints {
            connections_managers.insert(
                endpoint,
                ConnectionManager::new(
                    endpoint,
                    config.reconnection_backoff,
                    config.maximum_reconnection_backoff,
                )
                .await,
            );
        }

        Client {
            connections_managers,
        }
    }

    pub async fn send(&mut self, address: SocketAddr, message: &[u8]) -> io::Result<()> {
        if !self.connections_managers.contains_key(&address) {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "address not in the list - dynamic connections not yet supported",
            ));
        }

        // let (tx, rx) = oneshot::channel();

        // to optimize later by using channels and separate tokio tasks for each connection handler
        // because right now say we want to write to addresses A and B -
        // We have to wait until we're done dealing with A before we can do anything with B
        self.connections_managers
            .get_mut(&address)
            .unwrap()
            .send(&message)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::*;
    use std::str;
    use std::{env, time};
    use tokio::prelude::*;

    const CLOSE_MESSAGE: [u8; 3] = [0, 0, 0];

    struct DummyServer {
        received_buf: Vec<Vec<u8>>,
    }

    impl DummyServer {
        fn new() -> Self {
            DummyServer {
                received_buf: Vec::new(),
            }
        }

        fn get_received(&self) -> Vec<Vec<u8>> {
            self.received_buf.clone()
        }

        // this is only used in tests so slightly higher logging levels are fine
        async fn listen_until(mut self, addr: SocketAddr, close_message: &[u8]) -> Self {
            let mut listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            loop {
                let mut buf = [0u8; 1024];
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
        let num_test_threads = env::var("RUST_TEST_THREADS").expect("Number of test threads must be set to 1 to prevent tests interacting with themselves! (RUST_TEST_THREADS=1)");
        assert_eq!(num_test_threads, "1", "Number of test threads must be set to 1 to prevent tests interacting with themselves! (RUST_TEST_THREADS=1)");

        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let addr = "127.0.0.1:5000".parse().unwrap();
        let reconnection_backoff = Duration::from_secs(1);
        let client_config =
            Config::new(vec![addr], reconnection_backoff, 10 * reconnection_backoff);

        let messages_to_send = vec![b"foomp1", b"foomp2"];
        let finished_dummy_server_future =
            rt.spawn(DummyServer::new().listen_until(addr, CLOSE_MESSAGE.as_ref()));

        let mut c = rt.block_on(Client::new(client_config));

        for msg in &messages_to_send {
            rt.block_on(c.send(addr, *msg)).unwrap();
            rt.block_on(
                async move { tokio::time::delay_for(time::Duration::from_millis(50)).await },
            );
        }

        // kill server
        rt.block_on(c.send(addr, CLOSE_MESSAGE.as_ref())).unwrap();
        rt.block_on(async move { tokio::time::delay_for(time::Duration::from_millis(50)).await });

        // try to send - go into reconnection
        let post_kill_message = b"new foomp";

        // we are trying to send to killed server
        assert!(rt.block_on(c.send(addr, post_kill_message)).is_err());
        // the server future should have already been resolved
        let received_messages = rt
            .block_on(finished_dummy_server_future)
            .unwrap()
            .get_received();

        assert_eq!(received_messages, messages_to_send);

        let new_server_future =
            rt.spawn(DummyServer::new().listen_until(addr, CLOSE_MESSAGE.as_ref()));

        // keep sending after we leave reconnection backoff and reconnect
        loop {
            if rt.block_on(c.send(addr, post_kill_message)).is_ok() {
                break;
            }
            rt.block_on(
                async move { tokio::time::delay_for(time::Duration::from_millis(50)).await },
            );
        }
        rt.block_on(async move { tokio::time::delay_for(time::Duration::from_millis(50)).await });

        // kill the server to ensure it actually got the message
        rt.block_on(c.send(addr, CLOSE_MESSAGE.as_ref())).unwrap();
        let new_received_messages = rt.block_on(new_server_future).unwrap().get_received();
        assert_eq!(post_kill_message.to_vec(), new_received_messages[0]);
    }

    #[test]
    fn server_receives_all_sent_messages_when_up() {
        let num_test_threads = env::var("RUST_TEST_THREADS").expect("Number of test threads must be set to 1 to prevent tests interacting with themselves! (RUST_TEST_THREADS=1)");
        assert_eq!(num_test_threads, "1", "Number of test threads must be set to 1 to prevent tests interacting with themselves! (RUST_TEST_THREADS=1)");

        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let addr = "127.0.0.1:5000".parse().unwrap();
        let reconnection_backoff = Duration::from_secs(2);
        let client_config =
            Config::new(vec![addr], reconnection_backoff, 10 * reconnection_backoff);

        let messages_to_send = vec![b"foomp1", b"foomp2"];
        let finished_dummy_server_future =
            rt.spawn(DummyServer::new().listen_until(addr, CLOSE_MESSAGE.as_ref()));

        let mut c = rt.block_on(Client::new(client_config));

        for msg in &messages_to_send {
            rt.block_on(c.send(addr, *msg)).unwrap();
            rt.block_on(
                async move { tokio::time::delay_for(time::Duration::from_millis(50)).await },
            );
        }

        rt.block_on(c.send(addr, CLOSE_MESSAGE.as_ref())).unwrap();

        // the server future should have already been resolved
        let received_messages = rt
            .block_on(finished_dummy_server_future)
            .unwrap()
            .get_received();

        assert_eq!(received_messages, messages_to_send);
    }
}
