use futures::future::BoxFuture;
use futures::FutureExt;
use log::*;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub(crate) struct ConnectionReconnector<'a> {
    address: SocketAddr,
    connection: BoxFuture<'a, io::Result<tokio::net::TcpStream>>,

    current_retry_attempt: u32,

    current_backoff_delay: tokio::time::Delay,
    maximum_reconnection_backoff: Duration,

    initial_reconnection_backoff: Duration,
}

impl<'a> ConnectionReconnector<'a> {
    pub(crate) fn new(
        address: SocketAddr,
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
    ) -> ConnectionReconnector<'a> {
        ConnectionReconnector {
            address,
            connection: tokio::net::TcpStream::connect(address).boxed(),
            current_backoff_delay: tokio::time::delay_for(Duration::new(0, 0)), // if we can re-establish connection on first try without any backoff that's perfect
            current_retry_attempt: 0,
            maximum_reconnection_backoff,
            initial_reconnection_backoff,
        }
    }
}

impl<'a> Future for ConnectionReconnector<'a> {
    type Output = tokio::net::TcpStream;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // see if we are still in exponential backoff
        if Pin::new(&mut self.current_backoff_delay)
            .poll(cx)
            .is_pending()
        {
            return Poll::Pending;
        };

        // see if we managed to resolve the connection yet
        match Pin::new(&mut self.connection).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => {
                warn!(
                    "we failed to re-establish connection to {} - {:?} (attempt {})",
                    self.address, e, self.current_retry_attempt
                );
                self.current_retry_attempt += 1;

                // we failed to re-establish connection - continue exponential backoff
                let next_delay = std::cmp::min(
                    self.maximum_reconnection_backoff,
                    2_u32.pow(self.current_retry_attempt) * self.initial_reconnection_backoff,
                );

                self.current_backoff_delay
                    .reset(tokio::time::Instant::now() + next_delay);

                self.connection = tokio::net::TcpStream::connect(self.address).boxed();

                Poll::Pending
            }
            Poll::Ready(Ok(conn)) => Poll::Ready(conn),
        }
    }
}
