// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{config::NoiseConfig, error::NoiseError};
use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};
use nym_crypto::asymmetric::x25519;
use pin_project::pin_project;
use snow::{Builder, HandshakeState, TransportState};
use std::cmp::min;
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

pub(crate) type Psk = [u8; 32];

/// Wrapper around a TcpStream
#[pin_project]
pub struct NoiseStream {
    #[pin]
    inner_stream: Framed<TcpStream, LengthDelimitedCodec>,
    handshake: Option<HandshakeState>,
    noise: Option<TransportState>,
    dec_buffer: BytesMut,
}

impl NoiseStream {
    pub(crate) fn new_initiator(
        inner_stream: TcpStream,
        config: &NoiseConfig,
        remote_pub_key: &x25519::PublicKey,
        psk: &Psk,
    ) -> Result<NoiseStream, NoiseError> {
        let handshake = Builder::new(config.pattern.as_noise_params())
            .local_private_key(config.local_key.private_key().as_bytes())
            .remote_public_key(&remote_pub_key.to_bytes())
            .psk(config.pattern.psk_position(), psk)
            .build_initiator()?;
        Ok(NoiseStream::new_inner(inner_stream, handshake))
    }

    pub(crate) fn new_responder(
        inner_stream: TcpStream,
        config: &NoiseConfig,
        psk: &Psk,
    ) -> Result<NoiseStream, NoiseError> {
        let handshake = Builder::new(config.pattern.as_noise_params())
            .local_private_key(config.local_key.private_key().as_bytes())
            .psk(config.pattern.psk_position(), psk)
            .build_responder()?;
        Ok(NoiseStream::new_inner(inner_stream, handshake))
    }

    fn new_inner(inner_stream: TcpStream, handshake: HandshakeState) -> NoiseStream {
        NoiseStream {
            inner_stream: LengthDelimitedCodec::builder()
                .length_field_type::<u16>()
                .new_framed(inner_stream),
            handshake: Some(handshake),
            noise: None,
            dec_buffer: BytesMut::with_capacity(MAXMSGLEN),
        }
    }

    pub(crate) async fn perform_handshake(mut self) -> Result<Self, NoiseError> {
        //Check if we are in the correct state
        let Some(mut handshake) = self.handshake.take() else {
            return Err(NoiseError::IncorrectStateError);
        };

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

        let pending = match projected_self.inner_stream.poll_next(cx) {
            Poll::Pending => {
                //no new data, a return value of Poll::Pending means the waking is already scheduled
                //Nothing new to decrypt, only check if we can return something from dec_storage, happens after
                true
            }

            Poll::Ready(Some(Ok(noise_msg))) => {
                // We have a new noise msg
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
                false
            }

            Poll::Ready(Some(Err(err))) => return Poll::Ready(Err(err)),

            Poll::Ready(None) => {
                //Stream is done, we might still have data in the buffer though, happens afterwards
                false
            }
        };

        // Checking if there is something to return from the buffer
        let read_len = min(buf.remaining(), projected_self.dec_buffer.len());
        if read_len > 0 {
            buf.put_slice(&projected_self.dec_buffer.split_to(read_len));
            return Poll::Ready(Ok(()));
        }

        // buf.remaining == 0 or nothing in the buffer, we must return the value we had from the inner_stream
        if pending {
            //If we end up here, it means the previous poll_next was pending as well, hence waking is already scheduled
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
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
                match projected_self
                    .inner_stream
                    .as_mut()
                    .start_send(noise_buf.into())
                {
                    Ok(()) => match projected_self.inner_stream.poll_flush(cx) {
                        Poll::Pending => Poll::Pending, // A return value of Poll::Pending means the waking is already scheduled
                        Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                        Poll::Ready(Ok(())) => Poll::Ready(Ok(buf.len())),
                    },
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
