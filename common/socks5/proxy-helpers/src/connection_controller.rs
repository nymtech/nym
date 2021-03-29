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
use futures::StreamExt;
use log::*;
use ordered_buffer::{OrderedMessage, OrderedMessageBuffer};
use socks5_requests::ConnectionId;
use std::collections::{HashMap, HashSet};

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
    Insert(ConnectionId, ConnectionSender),
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

    // TODO: this will need to be either completely removed (from code) or periodically cleaned
    // to avoid memory issues
    recently_closed: HashSet<ConnectionId>,

    // TODO: this can potentially be abused to ddos and kill provider. Not sure at this point
    // how to handle it more gracefully

    // buffer for messages received before connection was established due to mixnet being able to
    // un-order messages. Note we don't ever expect to have more than 1-2 messages per connection here
    pending_messages: HashMap<ConnectionId, Vec<(Vec<u8>, bool)>>,
}

impl Controller {
    pub fn new() -> (Self, ControllerSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            Controller {
                active_connections: HashMap::new(),
                receiver,
                recently_closed: HashSet::new(),
                pending_messages: HashMap::new(),
            },
            sender,
        )
    }

    fn insert_connection(&mut self, conn_id: ConnectionId, connection_sender: ConnectionSender) {
        let active_connection = ActiveConnection {
            is_closed: false,
            connection_sender: Some(connection_sender),
            ordered_buffer: OrderedMessageBuffer::new(),
        };
        if let Some(_active_conn) = self.active_connections.insert(conn_id, active_connection) {
            error!("Received a duplicate 'Connect'!")
        } else {
            // check if there were any pending messages
            if let Some(pending) = self.pending_messages.remove(&conn_id) {
                debug!("There were some pending messages for {}", conn_id);
                for (payload, is_closed) in pending {
                    self.send_to_connection(conn_id, payload, is_closed)
                }
            }
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
            if !payload.is_empty() {
                active_connection.write_to_buf(payload);
            } else if !is_closed {
                error!("Tried to write an empty message to a not-closing connection. Please let us know if you see this message");
            }
            // if messages get unordered, make sure we don't lose information about
            // remote socket getting closed!
            active_connection.is_closed |= is_closed;

            if let Some(payload) = active_connection.read_from_buf() {
                if let Err(err) = active_connection
                    .connection_sender
                    .as_mut()
                    .unwrap()
                    .unbounded_send(ConnectionMessage {
                        payload,
                        socket_closed: active_connection.is_closed,
                    })
                {
                    error!("WTF IS THIS: {:?}", err);
                }

                // TODO: ABOVE UNWRAP CAUSED A CRASH IN A NORMAL USE!!!!
                // TODO:
                // TODO: surprisingly it only happened on socks client, never on nSP
                // TODO:
                // TODO:
                // TODO:
                // TODO:
            }
        } else if !self.recently_closed.contains(&conn_id) {
            debug!("Received a 'Send' before 'Connect' - going to buffer the data");
            let pending = self
                .pending_messages
                .entry(conn_id)
                .or_insert_with(Vec::new);
            pending.push((payload, is_closed));
        } else if !is_closed {
            error!(
                "Tried to write to closed connection ({} bytes were 'lost)",
                payload.len()
            );
        } else {
            debug!("Tried to write to closed connection, but remote is already closed")
        }
    }

    pub async fn run(&mut self) {
        while let Some(command) = self.receiver.next().await {
            match command {
                ControllerCommand::Send(conn_id, data, is_closed) => {
                    self.send_to_connection(conn_id, data, is_closed)
                }
                ControllerCommand::Insert(conn_id, sender) => {
                    self.insert_connection(conn_id, sender)
                }
                ControllerCommand::Remove(conn_id) => self.remove_connection(conn_id),
            }
        }
    }
}
