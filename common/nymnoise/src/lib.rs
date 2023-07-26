// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{BufMut, BytesMut};
use log::*;
use nym_topology::NymTopology;
use pin_project::pin_project;
use sha2::{Digest, Sha256};
use snow::error::Prerequisite;
use snow::Builder;
use snow::Error as NoiseError;
use snow::TransportState;
use std::io;
use std::pin::Pin;
use std::task::Poll;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::ReadBuf;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

const NOISE_HS_PATTERN: &str = "Noise_XKpsk3_25519_AESGCM_SHA256";
const MAXMSGLEN: usize = 65535;
const HEADER_SIZE: usize = 2;

/// Wrapper around a TcpStream
#[pin_project]
pub struct NoiseStream {
    #[pin]
    inner_stream: TcpStream,
    noise: TransportState,
    storage: BytesMut,
}

impl NoiseStream {
    fn new(inner_stream: TcpStream, noise: TransportState) -> NoiseStream {
        NoiseStream {
            inner_stream,
            noise,
            storage: BytesMut::with_capacity(MAXMSGLEN + HEADER_SIZE),
        }
    }
}

impl AsyncRead for NoiseStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let projected_self = self.project();
        let mut inner_vec = vec![0u8; MAXMSGLEN + HEADER_SIZE];
        let mut noise_buf = ReadBuf::new(&mut inner_vec);

        match projected_self.inner_stream.poll_read(cx, &mut noise_buf) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(())) => {
                let bytes_read = [
                    &projected_self.storage[..projected_self.storage.len()],
                    noise_buf.filled(),
                ]
                .concat();
                projected_self.storage.clear();

                //We can't read the length
                if bytes_read.len() < HEADER_SIZE {
                    projected_self.storage.put_slice(&bytes_read);
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                let msg_len = ((bytes_read[0] as usize) << 8) + (bytes_read[1] as usize);
                //we can't read the whole message
                if bytes_read.len() < HEADER_SIZE + msg_len {
                    projected_self.storage.put_slice(&bytes_read);
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                //we have a full Noise message available
                let mut payload = vec![0u8; MAXMSGLEN];
                println!("Read : {:?}", &noise_buf);
                let len = projected_self
                    .noise
                    .read_message(
                        &bytes_read[HEADER_SIZE..HEADER_SIZE + msg_len],
                        &mut payload,
                    )
                    .unwrap();
                buf.put_slice(&payload[..len]);
                projected_self
                    .storage
                    .put_slice(&bytes_read[HEADER_SIZE + msg_len..]);
                return Poll::Ready(Ok(()));
            }
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
        }
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
        let len = projected_self
            .noise
            .write_message(buf, &mut noise_buf)
            .unwrap();
        println!("Will send : {:?}", &noise_buf[..len]);
        let to_send = [
            &[(buf.len() >> 8) as u8, (buf.len() & 0xff) as u8],
            &noise_buf[..len],
        ]
        .concat();
        let res = projected_self.inner_stream.poll_write(cx, &to_send);
        println!("Sent : {:?}", res);
        res
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
    mut conn: TcpStream,
    topology: &NymTopology,
    local_public_key: &[u8],
    local_private_key: &[u8],
) -> Result<NoiseStream, NoiseError> {
    debug!("Perform Noise Handshake, initiator side");

    //Get init material
    let responder_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Prerequisite::RemotePublicKey.into());
        }
    };
    let remote_pub_key = match topology.find_node_key_by_mix_host(responder_addr) {
        Some(pub_key) => pub_key.to_bytes(),
        None => {
            error!(
                "Cannot find public key for node with address {:?}",
                responder_addr
            );
            return Err(Prerequisite::RemotePublicKey.into());
        }
    };
    let secret = [local_public_key, &remote_pub_key].concat();
    let secret_hash = Sha256::digest(secret);

    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let mut handshake = builder
        .local_private_key(local_private_key)
        .remote_public_key(&remote_pub_key)
        .psk(3, &secret_hash)
        .build_initiator()?;

    //Actual Handshake
    let mut buf = vec![0u8; MAXMSGLEN];
    // -> e, es
    let len = handshake.write_message(&[], &mut buf).unwrap();
    send(&mut conn, &buf[..len]).await;

    // <- e, ee
    handshake
        .read_message(&recv(&mut conn).await.unwrap(), &mut buf)
        .unwrap();

    // -> s, se, psk
    let len = handshake.write_message(&[], &mut buf).unwrap();
    send(&mut conn, &buf[..len]).await;

    let noise = handshake.into_transport_mode().unwrap();

    Ok(NoiseStream::new(conn, noise))
}

pub async fn upgrade_noise_responder(
    mut conn: TcpStream,
    topology: &NymTopology,
    local_public_key: &[u8],
    local_private_key: &[u8],
) -> Result<NoiseStream, NoiseError> {
    debug!("Perform Noise Handshake, responder side");

    //Get init material
    let initiator_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Prerequisite::RemotePublicKey.into());
        }
    };
    let remote_pub_key = match topology.find_node_key_by_mix_host(initiator_addr) {
        Some(pub_key) => pub_key.to_bytes(),
        None => {
            error!(
                "Cannot find public key for node with address {:?}",
                initiator_addr
            );
            return Err(Prerequisite::RemotePublicKey.into());
        }
    };
    let secret = [&remote_pub_key, local_public_key].concat();
    let secret_hash = Sha256::digest(secret);

    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let mut handshake = builder
        .local_private_key(local_private_key)
        .psk(3, &secret_hash)
        .build_responder()?;

    //Actual Handshake
    let mut buf = vec![0u8; MAXMSGLEN];
    // <- e, es
    handshake
        .read_message(&recv(&mut conn).await.unwrap(), &mut buf)
        .unwrap();

    // -> e, ee
    let len = handshake.write_message(&[], &mut buf).unwrap();
    send(&mut conn, &buf[..len]).await;

    // <- s, se, psk
    handshake
        .read_message(&recv(&mut conn).await.unwrap(), &mut buf)
        .unwrap();

    let noise = handshake.into_transport_mode().unwrap();

    Ok(NoiseStream::new(conn, noise))
}

/// Hyper-basic stream transport receiver. 16-bit BE size followed by payload.
async fn recv(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut msg_len_buf = [0u8; HEADER_SIZE];
    stream.read_exact(&mut msg_len_buf).await?;
    let msg_len = ((msg_len_buf[0] as usize) << 8) + (msg_len_buf[1] as usize);
    let mut msg = vec![0u8; msg_len];
    stream.read_exact(&mut msg[..]).await?;
    Ok(msg)
}

/// Hyper-basic stream transport sender. 16-bit BE size followed by payload.
async fn send(stream: &mut TcpStream, buf: &[u8]) {
    let msg_len_buf = [(buf.len() >> 8) as u8, (buf.len() & 0xff) as u8];
    stream.write_all(&msg_len_buf).await.unwrap();
    stream.write_all(buf).await.unwrap();
}
