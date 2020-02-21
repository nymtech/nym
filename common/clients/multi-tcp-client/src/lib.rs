use futures::task::{Context, Poll};
use futures::{AsyncWrite, AsyncWriteExt};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::str;
use tokio::prelude::*;

struct ConnectionWriter {
    connection: tokio::net::TcpStream,
}

impl ConnectionWriter {
    fn new(conn: tokio::net::TcpStream) -> Self {
        ConnectionWriter { connection: conn }
    }
}

impl Drop for ConnectionWriter {
    fn drop(&mut self) {
        // try to cleanly shutdown connection on going out of scope
        if let Err(e) = self.connection.shutdown(std::net::Shutdown::Both) {
            eprintln!("Failed to cleanly shutdown the connection - {:?}", e);
        }
    }
}

impl AsyncWrite for ConnectionWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        use tokio::io::AsyncWrite;

        let mut read_buf = [0; 1];
        match Pin::new(&mut self.connection).poll_read(cx, &mut read_buf) {
            // at least try the obvious check if connection is definitely down
            // can't do more than that
            Poll::Ready(Ok(n)) if n == 0 => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "trying to write to closed connection",
            ))),
            _ => Pin::new(&mut self.connection).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use tokio::io::AsyncWrite;
        Pin::new(&mut self.connection).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use tokio::io::AsyncWrite;
        Pin::new(&mut self.connection).poll_shutdown(cx)
    }
}

pub struct Client {
    connections_writers: HashMap<SocketAddr, ConnectionWriter>,
}

impl Client {
    pub async fn new(endpoints: Vec<SocketAddr>) -> Client {
        let mut connections_writers = HashMap::new();
        for endpoint in endpoints {
            connections_writers.insert(
                endpoint,
                ConnectionWriter::new(tokio::net::TcpStream::connect(endpoint).await.unwrap()),
            );
        }

        Client {
            connections_writers,
        }
    }

    pub async fn send(&mut self, address: SocketAddr, message: &[u8]) -> io::Result<()> {
        println!("sending {:?}", str::from_utf8(message));
        if !self.connections_writers.contains_key(&address) {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "address not in the list",
            ));
        }

        // to optimize later by using channels and separate tokio tasks for each connection handler
        // because right now say we want to write to addresses A and B -
        // We have to wait until we're done dealing with A before we can do anything with B
        if let Err(e) = self
            .connections_writers
            .get_mut(&address)
            .unwrap()
            .write_all(&message)
            .await
        {
            println!(
                "Failed to write to socket - {:?}. Presumably we need to reconnect!",
                e
            );
            // TODO: reconnection
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time;

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

        async fn listen_until(mut self, addr: SocketAddr, close_message: &[u8]) -> Self {
            let mut listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            println!("started");

            let (mut socket, _) = listener.accept().await.unwrap();
            println!("connected");
            loop {
                let mut buf = [0u8; 1024];
                match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => {
                        println!("Remote connection closed");
                        return self;
                    }
                    Ok(n) => {
                        println!("received ({}) - {:?}", n, str::from_utf8(buf[..n].as_ref()));

                        if buf[..n].as_ref() == close_message {
                            println!("closing...");
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
    fn server_receives_all_sent_messages_when_up() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let addr = "127.0.0.1:5000".parse().unwrap();

        let messages_to_send = vec![b"foomp1", b"foomp2"];
        let finished_dummy_server_future =
            rt.spawn(DummyServer::new().listen_until(addr, CLOSE_MESSAGE.as_ref()));

        let mut c = rt.block_on(Client::new(vec![addr]));

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
