// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: if it becomes too cumbersome, perhaps consider a more streamlined solution like tarpc

use crate::dkg::networking::codec::DkgCodec;
use futures::StreamExt;
use std::fmt::Display;
use std::net::SocketAddr;
use std::process;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_util::codec::Framed;

// note that we do not expect persistent connections between dealers, they should only really
// exist for the duration of a single message exchange
pub(crate) struct Listener<A> {
    address: A,
}

impl<A> Listener<A> {
    pub(crate) fn new(address: A) -> Self {
        Listener { address }
    }

    fn on_connect(&self, conn: TcpStream, remote: SocketAddr) {
        // TODO: here be a check to see if the connection originates from a known dealer
        tokio::spawn(async move {
            debug!("Starting connection handler for {}", remote);
            let mut framed_conn = Framed::new(conn, DkgCodec);
            while let Some(framed_dkg_request) = framed_conn.next().await {
                match framed_dkg_request {
                    Ok(framed_dkg_request) => {
                        todo!("handle packet")
                    }
                    Err(err) => {
                        warn!(
                        "The socket connection got corrupted with error: {:?}. Closing the socket",
                        err
                    );
                        break;
                    }
                }
            }

            debug!("Closing connection from {}", remote);
        });
    }

    async fn run(&mut self)
    where
        A: ToSocketAddrs + Display,
    {
        let listener = match TcpListener::bind(&self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {}.", self.address, err);
                process::exit(1);
            }
        };

        loop {
            match listener.accept().await {
                Ok((socket, remote_addr)) => self.on_connect(socket, remote_addr),
                Err(err) => warn!("Failed to accept incoming connection - {:?}", err),
            }
        }
    }
}
