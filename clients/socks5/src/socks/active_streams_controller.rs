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
use simple_socks5_requests::ConnectionId;
use std::collections::HashMap;
use tokio::stream::StreamExt;

pub(crate) type StreamResponseSender = mpsc::UnboundedSender<(Vec<u8>, bool)>;
pub(crate) type StreamResponseReceiver = mpsc::UnboundedReceiver<(Vec<u8>, bool)>;

pub(crate) type ControllerSender = mpsc::UnboundedSender<ControllerCommand>;
pub(crate) type ControllerReceiver = mpsc::UnboundedReceiver<ControllerCommand>;

pub enum ControllerCommand {
    Insert(ConnectionId, StreamResponseSender),
    Remove(ConnectionId),
    Send(ConnectionId, Vec<u8>, bool),
}

pub(super) struct Controller {
    active_streams: HashMap<ConnectionId, StreamResponseSender>,
    receiver: ControllerReceiver,
}

impl Controller {
    pub(crate) fn new() -> (Self, ControllerSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            Controller {
                active_streams: HashMap::new(),
                receiver,
            },
            sender,
        )
    }

    fn insert_connection(&mut self, conn_id: ConnectionId, sender: StreamResponseSender) {
        if self.active_streams.insert(conn_id, sender).is_some() {
            panic!("there is already an active request with the same id present - it's probably a bug!")
        }
    }

    fn remove_connection(&mut self, conn_id: ConnectionId) {
        if self.active_streams.remove(&conn_id).is_none() {
            error!(
                "tried to remove non-existing connection with id: {:?}",
                conn_id
            )
        }
    }

    fn send_to_connection(&mut self, conn_id: ConnectionId, data: Vec<u8>, is_closed: bool) {
        if let Some(sender) = self.active_streams.get_mut(&conn_id) {
            sender.unbounded_send((data, is_closed)).unwrap()
        } else {
            error!("no connection exists with id: {:?}", conn_id)
        }
    }

    pub(crate) async fn run(&mut self) {
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
