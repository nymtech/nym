// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use nym_topology::NymTopology;
use pin_project::pin_project;
use snow::error::Prerequisite;
use snow::Builder;
use snow::Error as NoiseError;
use std::pin::Pin;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

const NOISE_HS_PATTERN: &str = "Noise_XKpsk3_25519_AESGCM_SHA256";

static SECRET: &[u8] = b"i don't care for fidget spinners";

/// Wrapper around a TcpStream
//TODO SW : add psk3 to the protocol, requires topology at the receiver
#[pin_project]
pub struct NoiseStream {
    #[pin]
    inner_stream: TcpStream,
    //noise: TransportState,
}

impl NoiseStream {
    fn new(inner_stream: TcpStream) -> NoiseStream {
        NoiseStream {
            inner_stream,
            //   noise,
        }
    }
}

impl AsyncRead for NoiseStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut stream = self.project().inner_stream;
        Pin::new(&mut stream).poll_read(cx, buf)
    }
}
impl AsyncWrite for NoiseStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut stream = self.project().inner_stream;
        Pin::new(&mut stream).poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let mut stream = self.project().inner_stream;
        Pin::new(&mut stream).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let mut stream = self.project().inner_stream;
        Pin::new(&mut stream).poll_shutdown(cx)
    }
}

pub fn upgrade_noise_initiator(
    conn: TcpStream,
    topology: &NymTopology,
    local_private_key: &[u8],
) -> Result<NoiseStream, NoiseError> {
    debug!("Perform Noise Handshake, initiator side");
    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct

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

    let mut _handshake = builder
        .local_private_key(local_private_key)
        .remote_public_key(&remote_pub_key)
        .psk(3, SECRET)
        .build_initiator()?;

    Ok(NoiseStream::new(conn))
}
pub fn upgrade_noise_responder(
    conn: TcpStream,
    local_private_key: &[u8],
) -> Result<NoiseStream, NoiseError> {
    debug!("Perform Noise Handshake, responder side");
    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let mut _handshake = builder
        .local_private_key(local_private_key)
        .psk(3, SECRET)
        .build_responder()?;
    Ok(NoiseStream::new(conn))
}
