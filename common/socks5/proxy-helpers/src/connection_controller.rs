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
use socks5_requests::ConnectionId;
use std::collections::HashMap;
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
    Insert(ConnectionId, ConnectionSender),
    Remove(ConnectionId),
    Send(ConnectionId, Vec<u8>, bool),
}

/// Controller represents a way of managing multiple open connections that are used for socks5
/// proxy.
pub struct Controller {
    // TODO: this probably needs to be modified to somehow refer to the ordered buffer
    // as if the message are unordered and we received 'send' before 'connect', we will lose
    // packets
    active_connections: HashMap<ConnectionId, ConnectionSender>,
    receiver: ControllerReceiver,
}

impl Controller {
    pub fn new() -> (Self, ControllerSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            Controller {
                active_connections: HashMap::new(),
                receiver,
            },
            sender,
        )
    }

    fn insert_connection(&mut self, conn_id: ConnectionId, sender: ConnectionSender) {
        if self.active_connections.insert(conn_id, sender).is_some() {
            panic!("there is already an active request with the same id present - it's probably a bug!")
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
    }

    fn send_to_connection(&mut self, conn_id: ConnectionId, payload: Vec<u8>, is_closed: bool) {
        if let Some(sender) = self.active_connections.get_mut(&conn_id) {
            sender
                .unbounded_send(ConnectionMessage {
                    payload,
                    socket_closed: is_closed,
                })
                .unwrap()
        } else {
            error!("no connection exists with id: {:?}", conn_id);
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
