// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NoiseError;
use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};
use pin_project::pin_project;
use snow::{HandshakeState, TransportState};
use std::cmp::min;
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::task::Poll;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

const MAXMSGLEN: usize = 65535;
const TAGLEN: usize = 16;

#[derive(Default)]
pub enum NoisePattern {
    #[default]
    XKpsk3,
    IKpsk2,
}

impl NoisePattern {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::XKpsk3 => "Noise_XKpsk3_25519_AESGCM_SHA256",
            Self::IKpsk2 => "Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s", //Wireguard handshake (not exactly though)
        }
    }

    pub(crate) fn psk_position(&self) -> u8 {
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
    pub(crate) fn new(inner_stream: TcpStream, handshake: HandshakeState) -> NoiseStream {
        NoiseStream {
            inner_stream: LengthDelimitedCodec::builder()
                .length_field_type::<u16>()
                .new_framed(inner_stream),
            handshake: Some(handshake),
            noise: None,
            dec_buffer: VecDeque::with_capacity(MAXMSGLEN),
        }
    }

    pub(crate) async fn perform_handshake(mut self) -> Result<Self, NoiseError> {
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
                            Err(_) => return Poll::Ready(Err(io::ErrorKind::InvalidInput.into())),
                        }
                    }
                    None => return Poll::Ready(Err(io::ErrorKind::Other.into())),
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
                    None => return Poll::Ready(Err(io::ErrorKind::Other.into())),
                }) else {
                    return Poll::Ready(Err(io::ErrorKind::InvalidInput.into()));
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
