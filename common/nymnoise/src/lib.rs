// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use nym_topology::NymTopology;
use pin_project::pin_project;
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

static SECRET: &[u8] = b"i don't care for fidget spinners";
static PRIV_KEY: &[u8] = &[
    208, 217, 103, 180, 100, 15, 242, 137, 184, 247, 248, 193, 21, 66, 177, 79, 90, 131, 15, 134,
    145, 4, 45, 37, 215, 253, 227, 172, 113, 73, 97, 125,
];
static PUB_KEY: &[u8] = &[
    126, 100, 176, 138, 253, 249, 136, 187, 191, 200, 120, 5, 62, 218, 218, 73, 220, 60, 1, 179,
    49, 92, 253, 43, 91, 109, 18, 6, 88, 235, 123, 78,
];

/// Wrapper around a TcpStream
//TODO SW : add psk3 to the protocol, requires topology at the receiver
#[pin_project]
pub struct NoiseStream {
    #[pin]
    inner_stream: TcpStream,
    noise: TransportState,
}

impl NoiseStream {
    fn new(inner_stream: TcpStream, noise: TransportState) -> NoiseStream {
        NoiseStream {
            inner_stream,
            noise,
        }
    }
}

impl AsyncRead for NoiseStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut projected_self = self.project();
        return Pin::new(&mut projected_self.inner_stream).poll_read(cx, buf);
        let mut inner_vec = vec![0u8; 65535];
        let mut noise_buf = ReadBuf::new(&mut inner_vec);
        match Pin::new(&mut projected_self.inner_stream).poll_read(cx, &mut noise_buf) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(())) => {
                let mut payload = vec![0u8; 65535];
                println!("Read : {:?}", &noise_buf);
                let len = projected_self
                    .noise
                    .read_message(&noise_buf.filled(), &mut payload)
                    .unwrap();
                buf.put_slice(&payload[..len]);
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
        let mut projected_self = self.project();
        return Pin::new(&mut projected_self.inner_stream).poll_write(cx, &buf);
        //let mut stream = self.project().inner_stream;
        let mut noise_buf = vec![0u8; 65535];
        let len = projected_self
            .noise
            .write_message(buf, &mut noise_buf)
            .unwrap();
        println!("Will send : {:?}", &noise_buf[..len]);
        let res = Pin::new(&mut projected_self.inner_stream).poll_write(cx, &noise_buf[..len]);
        println!("Sent : {:?}", res);
        res
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut stream = self.project().inner_stream;
        Pin::new(&mut stream).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut stream = self.project().inner_stream;
        Pin::new(&mut stream).poll_shutdown(cx)
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
    let _secret = [local_public_key, &remote_pub_key].concat();

    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let mut handshake = builder
        .local_private_key(local_private_key)
        .remote_public_key(&remote_pub_key)
        .psk(3, SECRET)
        .build_initiator()?;

    //Actual Handshake
    let mut buf = vec![0u8; 65535];
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
    let _secret = [&remote_pub_key, local_public_key].concat();
    println!("Secret key : {:?}", local_private_key);

    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let mut handshake = builder
        .local_private_key(local_private_key)
        .psk(3, SECRET)
        .build_responder()?;

    //Actual Handshake
    let mut buf = vec![0u8; 65535];
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
    let mut msg_len_buf = [0u8; 2];
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
