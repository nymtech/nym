// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use futures::Sink;
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use log::*;
use nym_topology::NymTopology;
use pin_project::pin_project;
use sha2::{Digest, Sha256};
use snow::error::Prerequisite;
use snow::Builder;
use snow::Error;
use snow::HandshakeState;
use snow::TransportState;
use std::cmp::min;
use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind;
use std::num::TryFromIntError;
use std::pin::Pin;
use std::task::Poll;
use thiserror::Error;
use tokio::io::ReadBuf;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

const MAXMSGLEN: usize = 65535;
const TAGLEN: usize = 16;

#[derive(Error, Debug)]
pub enum NoiseError {
    #[error("encountered a Noise decryption error")]
    DecryptionError,

    #[error("encountered a Noise Protocol error - {0}")]
    ProtocolError(Error),
    #[error("encountered an IO error - {0}")]
    IoError(#[from] io::Error),

    #[error("Incorrect state")]
    IncorrectStateError,

    #[error("Handshake timeout")]
    HandshakeTimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("Handshake did not complete")]
    HandshakeError,

    #[error(transparent)]
    IntConversionError(#[from] TryFromIntError),
}

impl From<Error> for NoiseError {
    fn from(err: Error) -> Self {
        match err {
            Error::Decrypt => NoiseError::DecryptionError,
            err => NoiseError::ProtocolError(err),
        }
    }
}
#[derive(Default)]
pub enum NoisePattern {
    #[default]
    XKpsk3,
    IKpsk2,
}

impl NoisePattern {
    fn as_str(&self) -> &'static str {
        match self {
            Self::XKpsk3 => "Noise_XKpsk3_25519_AESGCM_SHA256",
            Self::IKpsk2 => "Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s", //Wireguard handshake (not exactly though)
        }
    }

    fn psk_position(&self) -> u8 {
        //automatic parsing, works for correct pattern, more convenient
        match self.as_str().find("psk") {
            Some(n) => {
                let psk_index = n + 3;
                let psk_char = self.as_str().chars().nth(psk_index).unwrap();
                psk_char.to_string().parse().unwrap()
                //if this fails, it means hardcoded pattern are wrong
            }
            None => 0,
        }
    }
}

/// Wrapper around a TcpStream
#[pin_project]
pub struct NoiseStream {
    #[pin]
    inner_stream: Framed<TcpStream, LengthDelimitedCodec>,
    handshake: Option<HandshakeState>,
    noise: Option<TransportState>,
    dec_buffer: VecDeque<u8>,
}

impl NoiseStream {
    fn new(inner_stream: TcpStream, handshake: HandshakeState) -> NoiseStream {
        NoiseStream {
            inner_stream: LengthDelimitedCodec::builder()
                .length_field_type::<u16>()
                .new_framed(inner_stream),
            handshake: Some(handshake),
            noise: None,
            dec_buffer: VecDeque::with_capacity(MAXMSGLEN),
        }
    }

    async fn perform_handshake(mut self) -> Result<Self, NoiseError> {
        //Check if we are in the correct state
        let Some(mut handshake) = self.handshake else {
            return Err(NoiseError::IncorrectStateError);
        };
        self.handshake = None;

        while !handshake.is_handshake_finished() {
            if handshake.is_my_turn() {
                self.send_handshake_msg(&mut handshake).await?;
            } else {
                self.recv_handshake_msg(&mut handshake).await?;
            }
        }

        self.noise = Some(handshake.into_transport_mode()?);
        Ok(self)
    }

    async fn send_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
    ) -> Result<(), NoiseError> {
        let mut buf = BytesMut::zeroed(MAXMSGLEN + TAGLEN);
        let len = handshake.write_message(&[], &mut buf)?;
        buf.truncate(len);
        self.inner_stream.send(buf.into()).await?;
        Ok(())
    }

    async fn recv_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
    ) -> Result<(), NoiseError> {
        match self.inner_stream.next().await {
            Some(Ok(msg)) => {
                let mut buf = vec![0u8; MAXMSGLEN];
                handshake.read_message(&msg, &mut buf)?;
                Ok(())
            }
            Some(Err(err)) => Err(NoiseError::IoError(err)),
            None => Err(NoiseError::HandshakeError),
        }
    }

    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, io::Error> {
        self.inner_stream.get_ref().peer_addr()
    }
}

impl AsyncRead for NoiseStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let projected_self = self.project();

        match projected_self.inner_stream.poll_next(cx) {
            Poll::Pending => {
                //no new data, waking is already scheduled.
                //Nothing new to decrypt, only check if we can return something from dec_storage, happens after
            }

            Poll::Ready(Some(Ok(noise_msg))) => {
                //We have a new moise msg
                let mut dec_msg = vec![0u8; MAXMSGLEN];
                let len = match projected_self.noise {
                    Some(transport_state) => {
                        match transport_state.read_message(&noise_msg, &mut dec_msg) {
                            Ok(len) => len,
                            Err(_) => return Poll::Ready(Err(ErrorKind::InvalidInput.into())),
                        }
                    }
                    None => return Poll::Ready(Err(ErrorKind::Other.into())),
                };
                projected_self.dec_buffer.extend(&dec_msg[..len]);
            }

            Poll::Ready(Some(Err(err))) => return Poll::Ready(Err(err)),

            //Stream is done, return Ok with nothing in buf
            Poll::Ready(None) => return Poll::Ready(Ok(())),
        }

        //check and return what we can
        let read_len = min(buf.remaining(), projected_self.dec_buffer.len());
        if read_len > 0 {
            buf.put_slice(
                &projected_self
                    .dec_buffer
                    .drain(..read_len)
                    .collect::<Vec<u8>>(),
            );
            return Poll::Ready(Ok(()));
        }

        //If we end up here, it must mean the previous poll_next was pending as well, otherwise something was returned. Hence waking is already scheduled
        Poll::Pending
    }
}

impl AsyncWrite for NoiseStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut projected_self = self.project();

        match projected_self.inner_stream.as_mut().poll_ready(cx) {
            Poll::Pending => Poll::Pending,

            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),

            Poll::Ready(Ok(())) => {
                let mut noise_buf = BytesMut::zeroed(MAXMSGLEN + TAGLEN);

                let Ok(len) = (match projected_self.noise {
                    Some(transport_state) => transport_state.write_message(buf, &mut noise_buf),
                    None => return Poll::Ready(Err(ErrorKind::Other.into())),
                }) else {
                    return Poll::Ready(Err(ErrorKind::InvalidInput.into()));
                };
                noise_buf.truncate(len);
                match projected_self.inner_stream.start_send(noise_buf.into()) {
                    Ok(()) => Poll::Ready(Ok(buf.len())),
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().inner_stream.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().inner_stream.poll_close(cx)
    }
}

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
    pattern: NoisePattern,
    local_public_key: Option<&[u8]>,
    local_private_key: &[u8],
    remote_pub_key: &[u8],
    epoch: u32,
) -> Result<NoiseStream, NoiseError> {
    trace!("Perform Noise Handshake, initiator side");

    //In case the local key cannot be known by the remote party, e.g. in a client-gateway connection
    let secret = [
        local_public_key.unwrap_or(&[]),
        remote_pub_key,
        &epoch.to_be_bytes(),
    ]
    .concat();
    let secret_hash = Sha256::digest(secret);

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(local_private_key)
        .remote_public_key(remote_pub_key)
        .psk(pattern.psk_position(), &secret_hash)
        .build_initiator()?;

    let noise_stream = NoiseStream::new(conn, handshake);

    noise_stream.perform_handshake().await
}

pub async fn upgrade_noise_initiator_with_topology(
    conn: TcpStream,
    pattern: NoisePattern,
    topology: &NymTopology,
    epoch: u32,
    local_public_key: &[u8],
    local_private_key: &[u8],
) -> Result<NoiseStream, NoiseError> {
    //Get init material
    let responder_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };
    let remote_pub_key = match topology.find_node_key_by_mix_host(responder_addr) {
        Some(pub_key) => pub_key.to_bytes(),
        None => {
            error!(
                "Cannot find public key for node with address {:?}",
                responder_addr
            );
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    upgrade_noise_initiator(
        conn,
        pattern,
        Some(local_public_key),
        local_private_key,
        &remote_pub_key,
        epoch,
    )
    .await
}

pub async fn upgrade_noise_responder(
    conn: TcpStream,
    pattern: NoisePattern,
    local_public_key: &[u8],
    local_private_key: &[u8],
    remote_pub_key: Option<&[u8]>,
    epoch: u32,
) -> Result<NoiseStream, NoiseError> {
    trace!("Perform Noise Handshake, responder side");

    //If the remote_key cannot be kwnown, e.g. in a client-gateway connection
    let secret = [
        remote_pub_key.unwrap_or(&[]),
        local_public_key,
        &epoch.to_be_bytes(),
    ]
    .concat();
    let secret_hash = Sha256::digest(secret);

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(local_private_key)
        .psk(pattern.psk_position(), &secret_hash)
        .build_responder()?;

    let noise_stream = NoiseStream::new(conn, handshake);

    noise_stream.perform_handshake().await
}

pub async fn upgrade_noise_responder_with_topology(
    conn: TcpStream,
    pattern: NoisePattern,
    topology: &NymTopology,
    epoch: u32,
    local_public_key: &[u8],
    local_private_key: &[u8],
) -> Result<NoiseStream, NoiseError> {
    //Get init material
    let initiator_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    //SW : for private gateway, we could try to perform the handshake without that key?
    let remote_pub_key = match topology.find_node_key_by_mix_host(initiator_addr) {
        Some(pub_key) => pub_key.to_bytes(),
        None => {
            error!(
                "Cannot find public key for node with address {:?}",
                initiator_addr
            );
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    upgrade_noise_responder(
        conn,
        pattern,
        local_public_key,
        local_private_key,
        Some(&remote_pub_key),
        epoch,
    )
    .await
}
