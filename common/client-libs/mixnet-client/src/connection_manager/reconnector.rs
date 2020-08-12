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

                // we failed to re-establish connection - continue exponential backoff

                // according to https://github.com/tokio-rs/tokio/issues/1953 there's an undocumented
                // limit of tokio delay of about 2 years.
                // let's ensure our delay is always on a sane side of being maximum 1 day.
                let maximum_sane_delay = Duration::from_secs(24 * 60 * 60);

                // MIN ( max_sane_delay, max_reconnection_backoff, 2^attempt * initial )
                let next_delay = std::cmp::min(
                    maximum_sane_delay,
                    std::cmp::min(
                        self.initial_reconnection_backoff
                            .checked_mul(2_u32.pow(self.current_retry_attempt))
                            .unwrap_or_else(|| self.maximum_reconnection_backoff),
                        self.maximum_reconnection_backoff,
                    ),
                );

                let now = self.current_backoff_delay.deadline();
                // this can't overflow now because next_delay is limited to one day...
                // ... well, unless you've been running the system for couple of decades without
                // any restarts....
                let next = now + next_delay;
                self.current_backoff_delay.reset(next);

                self.connection = tokio::net::TcpStream::connect(self.address).boxed();
                self.current_retry_attempt += 1;

                Poll::Pending
            }
            Poll::Ready(Ok(conn)) => Poll::Ready(conn),
        }
    }
}
