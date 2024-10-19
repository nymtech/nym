// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_ordered_buffer::{OrderedMessageBuffer, ReadContiguousData};
use nym_socks5_requests::{ConnectionId, SocketData};
use nym_task::connections::{ConnectionCommand, ConnectionCommandSender};
use nym_task::TaskClient;
use std::collections::{HashMap, HashSet};

/// A generic message produced after reading from a socket/connection.
///
/// A generic message produced after reading from a socket/connection that includes data that was
/// actually read alongside boolean indicating whether the connection got closed so that remote
/// could act accordingly.
#[derive(Debug)]
pub struct ConnectionMessage {
    pub payload: Vec<u8>,
    pub socket_closed: bool,
}

/// Channel responsible for sending data that was received from mix network into particular connection.
///
/// Data includes the actual payload that is to be written onto the connection
/// alongside boolean indicating whether the remote connection was closed after producing this message,
/// so that the local connection should also shut down.
pub type ConnectionSender = mpsc::UnboundedSender<ConnectionMessage>;

/// Receiver part of the [`ConnectionSender`]
pub type ConnectionReceiver = mpsc::UnboundedReceiver<ConnectionMessage>;

pub type ControllerSender = mpsc::UnboundedSender<ControllerCommand>;
pub type ControllerReceiver = mpsc::UnboundedReceiver<ControllerCommand>;

pub enum ControllerCommand {
    Insert {
        connection_id: ConnectionId,
        connection_sender: ConnectionSender,
    },
    Remove {
        connection_id: ConnectionId,
    },
    Send {
        data: SocketData,
    },
}

impl ControllerCommand {
    pub fn new_send(data: SocketData) -> Self {
        ControllerCommand::Send { data }
    }
}

struct ActiveConnection {
    is_closed: bool,
    closed_at_index: Option<u64>,
    connection_sender: Option<ConnectionSender>,
    ordered_buffer: OrderedMessageBuffer,
}

impl ActiveConnection {
    fn write_to_buf(&mut self, seq: u64, payload: Vec<u8>, is_closed: bool) {
        if is_closed {
            self.closed_at_index = Some(seq);
        }
        if let Err(err) = self.ordered_buffer.write(seq, payload) {
            error!("failed to write to the buffer: {err}")
        }
    }

    fn read_from_buf(&mut self) -> Option<ReadContiguousData> {
        self.ordered_buffer.read()
    }
}

#[derive(PartialEq, Eq)]
pub enum BroadcastActiveConnections {
    On,
    Off,
}

/// Controller represents a way of managing multiple open connections that are used for socks5
/// proxy.
pub struct Controller {
    active_connections: HashMap<ConnectionId, ActiveConnection>,
    receiver: ControllerReceiver,

    // TODO: this will need to be either completely removed (from code) or periodically cleaned
    // to avoid memory issues
    recently_closed: HashSet<ConnectionId>,

    // Broadcast closed connections
    client_connection_tx: ConnectionCommandSender,

    // TODO: this can potentially be abused to ddos and kill provider. Not sure at this point
    // how to handle it more gracefully

    // buffer for messages received before connection was established due to mixnet being able to
    // un-order messages. Note we don't ever expect to have more than 1-2 messages per connection here
    pending_messages: HashMap<ConnectionId, Vec<SocketData>>,

    shutdown: TaskClient,
}

impl Controller {
    pub fn new(
        client_connection_tx: ConnectionCommandSender,
        shutdown: TaskClient,
    ) -> (Self, ControllerSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            Controller {
                active_connections: HashMap::new(),
                receiver,
                recently_closed: HashSet::new(),
                client_connection_tx,
                pending_messages: HashMap::new(),
                shutdown,
            },
            sender,
        )
    }

    fn insert_connection(&mut self, conn_id: ConnectionId, connection_sender: ConnectionSender) {
        let active_connection = ActiveConnection {
            is_closed: false,
            connection_sender: Some(connection_sender),
            ordered_buffer: OrderedMessageBuffer::new(),
            closed_at_index: None,
        };
        if let Some(_active_conn) = self.active_connections.insert(conn_id, active_connection) {
            error!("Received a duplicate 'Connect'!")
        } else {
            // check if there were any pending messages
            if let Some(pending) = self.pending_messages.remove(&conn_id) {
                debug!("There were some pending messages for {conn_id}");
                for data in pending {
                    self.send_to_connection(data)
                }
            }
        }
    }

    fn remove_connection(&mut self, conn_id: ConnectionId) {
        debug!("Removing {conn_id} from controller");
        if self.active_connections.remove(&conn_id).is_none() {
            error!("tried to remove non-existing connection with id: {conn_id}",)
        }
        self.recently_closed.insert(conn_id);

        // Announce closed connections, currently used by the `OutQueueControl`.
        if let Err(err) = self
            .client_connection_tx
            .unbounded_send(ConnectionCommand::Close(conn_id))
        {
            if self.shutdown.is_shutdown_poll() {
                log::debug!("Failed to send: {err}");
            } else {
                log::error!("Failed to send: {err}");
            }
        }
    }

    fn send_to_connection(&mut self, message: SocketData) {
        let hdr = message.header;
        if let Some(active_connection) = self.active_connections.get_mut(&hdr.connection_id) {
            // always write to the buffer even if payload is empty (because it could have been the keep-alive message)
            active_connection.write_to_buf(hdr.seq, message.data, hdr.local_socket_closed);

            if let Some(payload) = active_connection.read_from_buf() {
                if let Some(closed_at_index) = active_connection.closed_at_index {
                    if payload.last_sequence >= closed_at_index {
                        active_connection.is_closed = true;
                    }
                }

                // however, don't send empty payload to the actual connection if it's not a close message
                // TODO: or should we?
                if payload.data.is_empty() && !active_connection.is_closed {
                    return;
                }

                if let Err(err) = active_connection
                    .connection_sender
                    .as_mut()
                    .unwrap()
                    .unbounded_send(ConnectionMessage {
                        payload: payload.data,
                        socket_closed: active_connection.is_closed,
                    })
                {
                    error!("failed to send on the active connection channel: {err}");
                }
            }
        } else if !self.recently_closed.contains(&hdr.connection_id) {
            debug!("Received a 'Send' before 'Connect' - going to buffer the data");
            let pending = self.pending_messages.entry(hdr.connection_id).or_default();
            pending.push(message);
        } else if !hdr.local_socket_closed {
            error!(
                "Tried to write to closed connection {} ({} bytes were 'lost)",
                hdr.connection_id,
                message.data.len()
            );
        } else {
            debug!(
                "Tried to write to closed connection {}, but remote is already closed",
                hdr.connection_id
            )
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                command = self.receiver.next() => match command {
                    Some(ControllerCommand::Send{data}) => {
                        self.send_to_connection(data)
                    }
                    Some(ControllerCommand::Insert{connection_id, connection_sender}) => {
                        self.insert_connection(connection_id, connection_sender)
                    }
                    Some(ControllerCommand::Remove{ connection_id }) => self.remove_connection(connection_id),
                    None => {
                        log::trace!("SOCKS5 Controller: Stopping since channel closed");
                        break;
                    }
                },
            }
        }
        self.shutdown.recv_timeout().await;
        log::debug!("SOCKS5 Controller: Exiting");
    }
}
