// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::VerlocError;
use crate::measurements::packet::{EchoPacket, ReplyPacket};
use bytes::{BufMut, BytesMut};
use futures::StreamExt;
use nym_crypto::asymmetric::identity;
use nym_task::TaskClient;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{io, process};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{Decoder, Encoder, Framed};
use tracing::{debug, error, info, trace, warn};

pub struct PacketListener {
    address: SocketAddr,
    connection_handler: Arc<ConnectionHandler>,
    shutdown: TaskClient,
}

impl PacketListener {
    pub fn new(
        address: SocketAddr,
        identity: Arc<identity::KeyPair>,
        shutdown: TaskClient,
    ) -> Self {
        PacketListener {
            address,
            connection_handler: Arc::new(ConnectionHandler { identity }),
            shutdown,
        }
    }
}

impl PacketListener {
    pub async fn run(self: Arc<Self>) {
        let listener = match TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!(
                    "Failed to bind to {}: {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?",
                    self.address
                );
                process::exit(1);
            }
        };

        info!("Started listening for echo packets on {}", self.address);

        let mut shutdown_listener = self.shutdown.clone();

        while !shutdown_listener.is_shutdown() {
            // cloning the arc as each accepted socket is handled in separate task
            let connection_handler = Arc::clone(&self.connection_handler);
            let mut handler_shutdown_listener = self.shutdown.clone();
            handler_shutdown_listener.disarm();

            tokio::select! {
                socket = listener.accept() => {
                    match socket {
                        Ok((socket, remote_addr)) => {
                            debug!("New verloc connection from {}", remote_addr);

                            tokio::spawn(connection_handler.handle_connection(socket, remote_addr, handler_shutdown_listener));
                        }
                        Err(err) => warn!("Failed to accept incoming connection - {err}"),
                    }
                },
                _ = shutdown_listener.recv() => {
                    trace!("PacketListener: Received shutdown");
                }
            }
        }
    }
}

struct ConnectionHandler {
    identity: Arc<identity::KeyPair>,
}

impl ConnectionHandler {
    // we don't have to do much, just construct a reply
    fn handle_echo_packet(&self, packet: EchoPacket) -> ReplyPacket {
        packet.construct_reply(self.identity.private_key())
    }

    pub(crate) async fn handle_connection(
        self: Arc<Self>,
        conn: TcpStream,
        remote: SocketAddr,
        mut shutdown_listener: TaskClient,
    ) {
        debug!("Starting connection handler for {:?}", remote);

        let mut framed_conn = Framed::new(conn, EchoPacketCodec);
        while !shutdown_listener.is_shutdown() {
            tokio::select! {
                biased;
               _ = shutdown_listener.recv() => {
                    trace!("ConnectionHandler: Shutdown received");
                }
                maybe_echo_packet = framed_conn.next() => {
                    // handle echo packet
                    let reply_packet = match maybe_echo_packet {
                        Some(Ok(echo_packet)) => self.handle_echo_packet(echo_packet),
                        Some(Err(err)) => {
                             debug!(
                                "The socket connection got corrupted with error: {err}. Closing the socket",
                            );
                            return;
                        }
                        None => {
                            debug!("The socket connection got terminated by the remote!");
                            return;
                        }
                    };

                    // write back the reply (note the lack of framing)
                    if let Err(err) = framed_conn
                        .get_mut()
                        .write_all(reply_packet.to_bytes().as_ref())
                        .await
                    {
                        debug!(
                            "Failed to write reply packet back to the sender - {}. Closing the socket on our end",
                            err
                        );
                        return;
                    }
                },
            }
        }
    }
}

#[derive(Debug, Error)]
enum EchoPacketCodecError {
    #[error("encountered io error {0}")]
    IoError(#[from] io::Error),

    #[error("failed to correctly decode an echo packet: {0}")]
    PacketRecoveryError(#[from] VerlocError),
}

// a super simple codec implemented for the convenience of Stream
// that also handles all eof for us
struct EchoPacketCodec;

impl Encoder<EchoPacket> for EchoPacketCodec {
    type Error = EchoPacketCodecError;

    fn encode(&mut self, item: EchoPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put(item.to_bytes().as_ref());
        Ok(())
    }
}

impl Decoder for EchoPacketCodec {
    type Item = EchoPacket;
    type Error = EchoPacketCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // reserve enough bytes if we can't read the frame
        if src.len() < EchoPacket::SIZE {
            src.reserve(EchoPacket::SIZE);
            return Ok(None);
        }

        let packet_bytes = src.split_to(EchoPacket::SIZE);

        let echo_packet = match EchoPacket::try_from_bytes(&packet_bytes) {
            Ok(packet) => packet,
            Err(err) => return Err(EchoPacketCodecError::PacketRecoveryError(err)),
        };

        // reserve enough bytes for the next frame
        src.reserve(EchoPacket::SIZE);

        Ok(Some(echo_packet))
    }
}
