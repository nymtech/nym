// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: if it becomes too cumbersome, perhaps consider a more streamlined solution like tarpc

use crate::dkg::networking::receiver::handler::ConnectionHandler;
use crate::dkg::state::StateAccessor;
use std::fmt::Display;
use std::net::SocketAddr;
use std::process;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};

// note that we do not expect persistent connections between dealers, they should only really
// exist for the duration of a single message exchange
pub struct Listener<A> {
    address: A,
    state_accessor: StateAccessor,
}

impl<A> Listener<A> {
    pub(crate) fn new(address: A, state_accessor: StateAccessor) -> Self {
        Listener {
            address,
            state_accessor,
        }
    }

    fn on_connect(&self, conn: TcpStream, remote: SocketAddr) {
        tokio::spawn(
            ConnectionHandler::new(self.state_accessor.clone(), conn, remote).handle_connection(),
        );
    }

    pub(crate) async fn run(&mut self)
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
