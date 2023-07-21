// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use nym_topology::NymTopology;
use pin_project::pin_project;
use snow::Error as NoiseError;
use snow::{params::NoiseParams, Builder, TransportState};
use std::pin::Pin;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

const NOISE_HS_PATTERN: &str = "Noise_XK_25519_AESGCM_SHA256";

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
) -> Result<NoiseStream, NoiseError> {
    debug!("Perform Noise Handshake, initiator side");
    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct

    let mut _handshake = builder
        .local_private_key(PRIV_KEY)
        .remote_public_key(PUB_KEY)
        //.psk(3, key)
        .build_initiator()?;

    Ok(NoiseStream::new(conn))
}
pub fn upgrade_noise_responder(conn: TcpStream) -> Result<NoiseStream, NoiseError> {
    debug!("Perform Noise Handshake, responder side");
    let builder = Builder::new(NOISE_HS_PATTERN.parse().unwrap()); //This cannot fail, hardcoded pattern must be correct
    let mut _handshake = builder
        .local_private_key(PRIV_KEY)
        //.psk(3, key)
        .build_responder()?;
    Ok(NoiseStream::new(conn))
}
