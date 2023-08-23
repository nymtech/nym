// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::ReadBuf;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

const NOISE_HS_PATTERN: &str = "Noise_XKpsk3_25519_AESGCM_SHA256";
const MAXMSGLEN: usize = 65535;
const TAGLEN: usize = 16;
const HEADER_SIZE: usize = 2;

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

/// Wrapper around a TcpStream
#[pin_project]
pub struct NoiseStream {
    #[pin]
    inner_stream: TcpStream,
    handshake: Option<HandshakeState>,
    noise: Option<TransportState>,
    enc_storage: VecDeque<u8>,
    dec_storage: VecDeque<u8>,
}

impl NoiseStream {
    fn new(inner_stream: TcpStream, handshake: HandshakeState) -> NoiseStream {
        NoiseStream {
            inner_stream,
            handshake: Some(handshake),
            noise: None,
            enc_storage: VecDeque::with_capacity(MAXMSGLEN + TAGLEN + HEADER_SIZE), //At least one message
            dec_storage: VecDeque::with_capacity(MAXMSGLEN),
        }
    }

    async fn send_handshake_msg(&mut self) -> Result<(), NoiseError> {
        let mut buf = vec![0u8; MAXMSGLEN];
        let len = match &mut self.handshake {
            Some(handshake_state) => handshake_state.write_message(&[], &mut buf)?,
            None => return Err(NoiseError::IncorrectStateError),
        };

        self.inner_stream.write_u16(len.try_into()?).await?; //len is always < 2^16, so it shouldn't fail
        self.inner_stream.write_all(&buf[..len]).await?;
        Ok(())
    }

    async fn recv_handshake_msg(&mut self) -> Result<(), NoiseError> {
        let msg_len = self.inner_stream.read_u16().await?;
        let mut msg = vec![0u8; msg_len.into()];
        self.inner_stream.read_exact(&mut msg[..]).await?;

        let mut buf = vec![0u8; MAXMSGLEN];
        match &mut self.handshake {
            Some(handshake_state) => handshake_state.read_message(&msg, &mut buf)?,
            None => return Err(NoiseError::IncorrectStateError),
        };
        Ok(())
    }

    fn into_transport_mode(mut self) -> Result<Self, NoiseError> {
        let Some(handshake) = self.handshake else {return Err(NoiseError::IncorrectStateError)};
        self.handshake = None;
        self.noise = Some(handshake.into_transport_mode()?);
        Ok(self)
    }
}

impl AsyncRead for NoiseStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let projected_self = self.project();
        let enc_storage = projected_self.enc_storage;
        let ready_to_read = projected_self.inner_stream.poll_read_ready(cx);

        match ready_to_read {
            Poll::Pending => {
                //no new data, waking is already scheduled.
                //Nothing new to decrypt, only check if we can return something from dec_storage, happens after
            }

            Poll::Ready(Ok(())) => {
                //Read what we can into enc_storage, decrypt what we can into dec_storage
                let mut tcp_buf = vec![0u8; MAXMSGLEN + HEADER_SIZE + TAGLEN];
                if let Ok(tcp_len) = projected_self.inner_stream.try_read(&mut tcp_buf) {
                    if tcp_len == 0 && projected_self.dec_storage.is_empty() {
                        //EOF
                        return Poll::Ready(Ok(()));
                    }
                    enc_storage.extend(&tcp_buf[..tcp_len]);
                    //we can at least read the length
                    while enc_storage.len() >= HEADER_SIZE {
                        let msg_len = ((enc_storage[0] as usize) << 8) + (enc_storage[1] as usize);

                        //no more messages to read
                        if enc_storage.len() < HEADER_SIZE + msg_len {
                            break;
                        }
                        //we have a full message to decrypt
                        //remove size
                        enc_storage.pop_front();
                        enc_storage.pop_front();

                        let noise_msg = enc_storage.drain(..msg_len).collect::<Vec<u8>>();
                        let mut dec_msg = vec![0u8; MAXMSGLEN];

                        let Ok(len) = (match projected_self.noise {
                            Some(transport_state) => transport_state.read_message(&noise_msg, &mut dec_msg),
                            None => return Poll::Ready(Err(ErrorKind::Other.into())),
                        }) else {
                            return Poll::Ready(Err(ErrorKind::InvalidInput.into()));
                        };
                        projected_self.dec_storage.extend(&dec_msg[..len]);
                    }
                }
            }

            //an error occured, let's return it right away
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
        }

        //check if we can return something
        let read_len = min(buf.remaining(), projected_self.dec_storage.len());
        if read_len > 0 {
            buf.put_slice(
                &projected_self
                    .dec_storage
                    .drain(..read_len)
                    .collect::<Vec<u8>>(),
            );
            return Poll::Ready(Ok(()));
        }

        //can't return anything, schedule the wakeup and return pending
        if let Poll::Ready(Ok(())) = projected_self.inner_stream.poll_read_ready(cx) {
            //we got data in the meantime, we can wake up immediately
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }
}
impl AsyncWrite for NoiseStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let projected_self = self.project();
        let mut noise_buf = vec![0u8; MAXMSGLEN];

        let Ok(len) = (match projected_self.noise {
            Some(transport_state) => transport_state.write_message(buf, &mut noise_buf),
            None => return Poll::Ready(Err(ErrorKind::Other.into())),
        }) else {
            return Poll::Ready(Err(ErrorKind::InvalidInput.into()));
        };
        let to_send = [&[(len >> 8) as u8, (len & 0xff) as u8], &noise_buf[..len]].concat();

        match projected_self.inner_stream.poll_write(cx, &to_send) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Ready(Ok(n)) => {
                //didn't send a thing, no problem for the underlying stream
                if n == 0 {
                    return Poll::Ready(Ok(0));
                }
                //we sent the whole thing, no problem for the underlying stream
                //We must guarantee that the return number is <= buf.len()
                if n == to_send.len() {
                    return Poll::Ready(Ok(n - HEADER_SIZE - TAGLEN));
                }
                //We didn't write the whole message, the stream will be corrupted
                error!(
                    "Partial write on Noise Stream, it will be corrupted - {}",
                    n
                );
                Poll::Ready(Err(ErrorKind::WriteZero.into()))
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
        self.project().inner_stream.poll_shutdown(cx)
    }
}

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
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

    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let handshake = builder
        .local_private_key(local_private_key)
        .remote_public_key(remote_pub_key)
        .psk(3, &secret_hash)
        .build_initiator()?;

    let mut noise_stream = NoiseStream::new(conn, handshake);

    tokio::time::timeout(Duration::from_secs(5), async {
        // -> e, es
        noise_stream.send_handshake_msg().await?;

        // <- e, ee
        noise_stream.recv_handshake_msg().await?;

        // -> s, se, psk

        noise_stream.send_handshake_msg().await
    })
    .await??;

    noise_stream.into_transport_mode()
}

pub async fn upgrade_noise_initiator_with_topology(
    conn: TcpStream,
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
        Some(local_public_key),
        local_private_key,
        &remote_pub_key,
        epoch,
    )
    .await
}

pub async fn upgrade_noise_responder(
    conn: TcpStream,
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

    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let handshake = builder
        .local_private_key(local_private_key)
        .psk(3, &secret_hash)
        .build_responder()?;

    let mut noise_stream = NoiseStream::new(conn, handshake);

    tokio::time::timeout(Duration::from_secs(5), async {
        //Actual Handshake
        // <- e, es
        noise_stream.recv_handshake_msg().await?;

        // -> e, ee
        noise_stream.send_handshake_msg().await?;

        // <- s, se, psk
        noise_stream.recv_handshake_msg().await
    })
    .await??;

    noise_stream.into_transport_mode()
}

pub async fn upgrade_noise_responder_with_topology(
    conn: TcpStream,
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
        local_public_key,
        local_private_key,
        Some(&remote_pub_key),
        epoch,
    )
    .await
}
