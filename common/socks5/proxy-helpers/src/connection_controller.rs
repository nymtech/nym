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

use futures::channel::mpsc;
use log::*;
use ordered_buffer::{OrderedMessage, OrderedMessageBuffer};
use socks5_requests::ConnectionId;
use std::collections::{HashMap, HashSet};
use tokio::stream::StreamExt;

/// A generic message produced after reading from a socket/connection. It includes data that was
/// actually read alongside boolean indicating whether the connection got closed so that
/// remote could act accordingly.
#[derive(Debug)]
pub struct ConnectionMessage {
    pub payload: Vec<u8>,
    pub socket_closed: bool,
}

/// Channel responsible for sending data that was received from mix network into particular connection.
/// Data includes the actual payload that is to be written onto the connection
/// alongside boolean indicating whether the remote connection was closed after producing this message,
/// so that the local connection should also shut down.
pub type ConnectionSender = mpsc::UnboundedSender<ConnectionMessage>;

/// Receiver part of the [`ConnectionSender`]
pub type ConnectionReceiver = mpsc::UnboundedReceiver<ConnectionMessage>;

pub type ControllerSender = mpsc::UnboundedSender<ControllerCommand>;
pub type ControllerReceiver = mpsc::UnboundedReceiver<ControllerCommand>;

pub enum ControllerCommand {
    Insert(ConnectionId, ConnectionSender, OrderedMessageBuffer),
    Remove(ConnectionId),
    Send(ConnectionId, Vec<u8>, bool),
}

struct ActiveConnection {
    is_closed: bool,
    connection_sender: Option<ConnectionSender>,
    ordered_buffer: OrderedMessageBuffer,
}

impl ActiveConnection {
    fn write_to_buf(&mut self, payload: Vec<u8>) {
        let ordered_message = match OrderedMessage::try_from_bytes(payload) {
            Ok(msg) => msg,
            Err(err) => {
                error!("Malformed ordered message - {:?}", err);
                return;
            }
        };
        self.ordered_buffer.write(ordered_message);
    }

    fn read_from_buf(&mut self) -> Option<Vec<u8>> {
        self.ordered_buffer.read()
    }
}

/// Controller represents a way of managing multiple open connections that are used for socks5
/// proxy.
pub struct Controller {
    active_connections: HashMap<ConnectionId, ActiveConnection>,
    receiver: ControllerReceiver,

    recently_closed: HashSet<ConnectionId>,
}

impl Controller {
    pub fn new() -> (Self, ControllerSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            Controller {
                active_connections: HashMap::new(),
                receiver,
                recently_closed: HashSet::new(),
            },
            sender,
        )
    }

    fn insert_connection(
        &mut self,
        conn_id: ConnectionId,
        connection_sender: ConnectionSender,
        ordered_buffer: OrderedMessageBuffer,
    ) {
        let active_connection = ActiveConnection {
            is_closed: false,
            connection_sender: Some(connection_sender),
            ordered_buffer,
        };
        if let Some(_active_conn) = self.active_connections.insert(conn_id, active_connection) {
            // we received 'Send' before 'connect' - drain what we currently accumulated into the fresh
            // buffer as this new one is going to be used for the connection
            // TODO: let's only do this if it's actually EVER fired
            error!("Presumably received 'Send' before 'Connect'!")
        }
    }

    fn remove_connection(&mut self, conn_id: ConnectionId) {
        debug!("Removing {} from controller", conn_id);
        if self.active_connections.remove(&conn_id).is_none() {
            error!(
                "tried to remove non-existing connection with id: {:?}",
                conn_id
            )
        }
        self.recently_closed.insert(conn_id);
    }

    fn send_to_connection(&mut self, conn_id: ConnectionId, payload: Vec<u8>, is_closed: bool) {
        if let Some(active_connection) = self.active_connections.get_mut(&conn_id) {
            active_connection.write_to_buf(payload);
            // if messages get unordered, make sure we don't lose information about
            // remote socket getting closed!
            active_connection.is_closed |= is_closed;

            if let Some(payload) = active_connection.read_from_buf() {
                active_connection
                    .connection_sender
                    .as_mut()
                    .unwrap()
                    .unbounded_send(ConnectionMessage {
                        payload,
                        socket_closed: active_connection.is_closed,
                    })
                    .unwrap()
            }
        } else {
            error!("no connection exists with id: {:?}", conn_id);
            warn!("'lost' bytes: {}", payload.len());
            if !self.recently_closed.contains(&conn_id) {
                // TODO: let's only do this if it's actually EVER fired
                error!("Presumably received 'Send' before 'Connect'! - First")
            }
        }
    }

    pub async fn run(&mut self) {
        while let Some(command) = self.receiver.next().await {
            match command {
                ControllerCommand::Send(conn_id, data, is_closed) => {
                    self.send_to_connection(conn_id, data, is_closed)
                }
                ControllerCommand::Insert(conn_id, sender, ordered_buffer) => {
                    self.insert_connection(conn_id, sender, ordered_buffer)
                }
                ControllerCommand::Remove(conn_id) => self.remove_connection(conn_id),
            }
        }
    }
}
