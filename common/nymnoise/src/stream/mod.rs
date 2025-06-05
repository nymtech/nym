// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{NoiseConfig, NoisePattern};
use crate::error::NoiseError;
use crate::psk_gen::generate_psk;
use crate::stream::codec::NymNoiseCodec;
use crate::stream::framing::NymNoiseFrame;
use bytes::{Bytes, BytesMut};
use futures::{Sink, SinkExt, Stream, StreamExt};
use nym_crypto::asymmetric::x25519;
use nym_noise_keys::NoiseVersion;
use snow::{Builder, HandshakeState, TransportState};
use std::io;
use std::pin::Pin;
use std::task::Poll;
use std::{cmp::min, task::ready};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::Framed;

mod codec;
mod framing;

const TAGLEN: usize = 16;
const HANDSHAKE_MAX_LEN: usize = 1024; // using this constant to limit the handshake's buffer size

pub(crate) type Psk = [u8; 32];

pub(crate) struct NoiseStreamBuilder<C> {
    inner_stream: Framed<C, NymNoiseCodec>,
}

impl<C> NoiseStreamBuilder<C> {
    pub(crate) fn new(inner_stream: C) -> Self
    where
        C: AsyncRead + AsyncWrite,
    {
        NoiseStreamBuilder {
            inner_stream: Framed::new(inner_stream, NymNoiseCodec::new()),
        }
    }

    async fn perform_initiator_handshake_inner(
        self,
        pattern: NoisePattern,
        local_private_key: impl AsRef<[u8]>,
        remote_pub_key: impl AsRef<[u8]>,
        psk: Psk,
        version: NoiseVersion,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let handshake = Builder::new(pattern.as_noise_params())
            .local_private_key(local_private_key.as_ref())
            .remote_public_key(remote_pub_key.as_ref())
            .psk(pattern.psk_position(), &psk)
            .build_initiator()?;

        self.perform_handshake(handshake, version, pattern).await
    }

    pub(crate) async fn perform_initiator_handshake(
        self,
        config: &NoiseConfig,
        version: NoiseVersion,
        remote_pub_key: x25519::PublicKey,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let psk = generate_psk(remote_pub_key, version)?;

        let timeout = config.timeout;
        tokio::time::timeout(
            timeout,
            self.perform_initiator_handshake_inner(
                config.pattern,
                config.local_key.private_key(),
                remote_pub_key,
                psk,
                version,
            ),
        )
        .await?
    }

    async fn perform_responder_handshake_inner(
        mut self,
        noise_pattern: NoisePattern,
        local_private_key: impl AsRef<[u8]>,
        local_pub_key: x25519::PublicKey,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        // 1. we read the first message from the initiator to establish noise version and pattern
        // and determine if we can continue with the handshake
        let initial_frame = self
            .inner_stream
            .next()
            .await
            .ok_or(NoiseError::IoError(io::ErrorKind::BrokenPipe.into()))??;

        if !initial_frame.is_handshake_message() {
            return Err(NoiseError::NonHandshakeMessageReceived);
        }

        let pattern = initial_frame.noise_pattern();

        // I can imagine we should be able to handle multiple patterns here, but I guess there's a reason a value is set in the config
        // but refactoring this shouldn't be too difficult
        if pattern != noise_pattern {
            return Err(NoiseError::UnexpectedNoisePattern {
                configured: noise_pattern.as_str(),
                received: pattern.as_str(),
            });
        }

        // 2. generate psk and handshake state
        let psk = generate_psk(local_pub_key, initial_frame.header.version)?;

        let mut handshake = Builder::new(pattern.as_noise_params())
            .local_private_key(local_private_key.as_ref())
            .psk(pattern.psk_position(), &psk)
            .build_responder()?;

        // update handshake state with initial frame
        let mut buf = BytesMut::zeroed(HANDSHAKE_MAX_LEN);
        handshake.read_message(&initial_frame.data, &mut buf)?;

        // 3. run handshake to completion
        self.perform_handshake(handshake, initial_frame.version(), pattern)
            .await
    }

    pub(crate) async fn perform_responder_handshake(
        self,
        config: &NoiseConfig,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let timeout = config.timeout;
        tokio::time::timeout(
            timeout,
            self.perform_responder_handshake_inner(
                config.pattern,
                config.local_key.private_key(),
                *config.local_key.public_key(),
            ),
        )
        .await?
    }

    async fn send_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
        version: NoiseVersion,
        pattern: NoisePattern,
    ) -> Result<(), NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let mut buf = BytesMut::zeroed(HANDSHAKE_MAX_LEN); // we're in the handshake, we can afford a smaller buffer
        let len = handshake.write_message(&[], &mut buf)?;
        buf.truncate(len);

        let frame = NymNoiseFrame::new_handshake_frame(buf.freeze(), version, pattern)?;
        self.inner_stream.send(frame).await?;
        Ok(())
    }

    async fn recv_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
        version: NoiseVersion,
        pattern: NoisePattern,
    ) -> Result<(), NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        match self.inner_stream.next().await {
            Some(Ok(frame)) => {
                // validate the frame
                if !frame.is_handshake_message() {
                    return Err(NoiseError::NonHandshakeMessageReceived);
                }
                if frame.version() != version {
                    return Err(NoiseError::UnexpectedHandshakeVersion {
                        initial: version,
                        received: frame.version(),
                    });
                }
                if frame.noise_pattern() != pattern {
                    return Err(NoiseError::UnexpectedNoisePattern {
                        configured: pattern.as_str(),
                        received: frame.noise_pattern().as_str(),
                    });
                }

                let mut buf = BytesMut::zeroed(HANDSHAKE_MAX_LEN); // we're in the handshake, we can afford a smaller buffer
                handshake.read_message(&frame.data, &mut buf)?;
                Ok(())
            }
            Some(Err(err)) => Err(err),
            None => Err(NoiseError::HandshakeError),
        }
    }

    async fn perform_handshake(
        mut self,
        mut handshake_state: HandshakeState,
        version: NoiseVersion,
        pattern: NoisePattern,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        while !handshake_state.is_handshake_finished() {
            if handshake_state.is_my_turn() {
                self.send_handshake_msg(&mut handshake_state, version, pattern)
                    .await?;
            } else {
                self.recv_handshake_msg(&mut handshake_state, version, pattern)
                    .await?;
            }
        }

        let transport = handshake_state.into_transport_mode()?;
        Ok(NoiseStream {
            inner_stream: self.inner_stream,
            negotiated_pattern: pattern,
            negotiated_version: version,
            transport,
            dec_buffer: Default::default(),
        })
    }
}

/// Wrapper around a TcpStream
pub struct NoiseStream<C> {
    inner_stream: Framed<C, NymNoiseCodec>,

    negotiated_pattern: NoisePattern,
    negotiated_version: NoiseVersion,

    transport: TransportState,
    dec_buffer: BytesMut,
}

impl<C> NoiseStream<C> {
    fn validate_data_frame(&self, frame: NymNoiseFrame) -> Result<Bytes, NoiseError> {
        if !frame.is_data_message() {
            return Err(NoiseError::NonDataMessageReceived);
        }
        // validate the frame
        if !frame.is_data_message() {
            return Err(NoiseError::NonDataMessageReceived);
        }
        if frame.version() != self.negotiated_version {
            return Err(NoiseError::UnexpectedDataVersion {
                initial: self.negotiated_version,
                received: frame.version(),
            });
        }
        if frame.noise_pattern() != self.negotiated_pattern {
            return Err(NoiseError::UnexpectedNoisePattern {
                configured: self.negotiated_pattern.as_str(),
                received: frame.noise_pattern().as_str(),
            });
        };

        Ok(frame.data)
    }

    fn poll_data_frame(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<io::Result<Bytes>>>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        match ready!(Pin::new(&mut self.inner_stream).poll_next(cx)) {
            None => Poll::Ready(None),
            Some(Err(err)) => Poll::Ready(Some(Err(err.naive_to_io_error()))),
            Some(Ok(frame)) => match self.validate_data_frame(frame) {
                Err(err) => Poll::Ready(Some(Err(err.naive_to_io_error()))),
                Ok(data) => Poll::Ready(Some(Ok(data))),
            },
        }
    }
}

impl<C> AsyncRead for NoiseStream<C>
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let pending = match self.poll_data_frame(cx) {
            Poll::Pending => {
                //no new data, a return value of Poll::Pending means the waking is already scheduled
                //Nothing new to decrypt, only check if we can return something from dec_storage, happens after
                true
            }

            Poll::Ready(Some(Ok(noise_msg))) => {
                // We have a new noise msg
                let mut dec_msg = BytesMut::zeroed(noise_msg.len() - TAGLEN);

                let len = match self.transport.read_message(&noise_msg, &mut dec_msg) {
                    Ok(len) => len,
                    Err(_) => return Poll::Ready(Err(io::ErrorKind::InvalidInput.into())),
                };

                self.dec_buffer.extend(&dec_msg[..len]);

                false
            }

            Poll::Ready(Some(Err(err))) => return Poll::Ready(Err(err)),

            Poll::Ready(None) => {
                //Stream is done, we might still have data in the buffer though, happens afterward
                false
            }
        };

        // Checking if there is something to return from the buffer
        let read_len = min(buf.remaining(), self.dec_buffer.len());
        if read_len > 0 {
            buf.put_slice(&self.dec_buffer.split_to(read_len));
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

impl<C> AsyncWrite for NoiseStream<C>
where
    C: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // returns on Poll::Pending and Poll:Ready(Err)
        ready!(Pin::new(&mut self.inner_stream).poll_ready(cx))
            .map_err(|err| err.naive_to_io_error())?;

        // we can send at most u16::MAX bytes in a frame, but we also have to include the tag when encoding
        let msg_len = min(u16::MAX as usize - TAGLEN, buf.len());

        // Ready to send, encrypting message
        let mut noise_buf = BytesMut::zeroed(msg_len + TAGLEN);

        let Ok(len) = self
            .transport
            .write_message(&buf[..msg_len], &mut noise_buf)
        else {
            return Poll::Ready(Err(io::ErrorKind::InvalidInput.into()));
        };
        noise_buf.truncate(len);

        let frame = NymNoiseFrame::new_data_frame(
            noise_buf.freeze(),
            self.negotiated_version,
            self.negotiated_pattern,
        )
        .map_err(|err| err.naive_to_io_error())?;

        // Tokio uses the same `start_send ` in their SinkWriter implementation. https://docs.rs/tokio-util/latest/src/tokio_util/io/sink_writer.rs.html#104
        match Pin::new(&mut self.inner_stream).start_send(frame) {
            Ok(()) => Poll::Ready(Ok(msg_len)),
            Err(e) => Poll::Ready(Err(e.naive_to_io_error())),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner_stream)
            .poll_flush(cx)
            .map_err(|err| err.naive_to_io_error())
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner_stream)
            .poll_close(cx)
            .map_err(|err| err.naive_to_io_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::x25519;
    use rand_chacha::rand_core::SeedableRng;
    use std::io::Error;
    use std::mem;
    use std::sync::Arc;
    use std::task::{Context, Waker};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::join;
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    fn mock_streams() -> (MockStream, MockStream) {
        let ch1 = Arc::new(Mutex::new(Default::default()));
        let ch2 = Arc::new(Mutex::new(Default::default()));

        (
            MockStream {
                inner: MockStreamInner {
                    tx: ch1.clone(),
                    rx: ch2.clone(),
                },
            },
            MockStream {
                inner: MockStreamInner { tx: ch2, rx: ch1 },
            },
        )
    }

    struct MockStream {
        inner: MockStreamInner,
    }

    #[allow(dead_code)]
    impl MockStream {
        fn unchecked_tx_data(&self) -> Vec<u8> {
            self.inner.tx.try_lock().unwrap().data.clone()
        }

        fn unchecked_rx_data(&self) -> Vec<u8> {
            self.inner.rx.try_lock().unwrap().data.clone()
        }
    }

    struct MockStreamInner {
        tx: Arc<Mutex<DataWrapper>>,
        rx: Arc<Mutex<DataWrapper>>,
    }

    #[derive(Default)]
    struct DataWrapper {
        data: Vec<u8>,
        waker: Option<Waker>,
    }

    impl AsyncRead for MockStream {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let mut inner = self.inner.rx.try_lock().unwrap();
            let data = mem::take(&mut inner.data);
            if data.is_empty() {
                inner.waker = Some(cx.waker().clone());
                return Poll::Pending;
            }

            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }

            buf.put_slice(&data);
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for MockStream {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            let mut inner = self.inner.tx.try_lock().unwrap();
            let len = buf.len();

            if !inner.data.is_empty() {
                assert!(inner.waker.is_none());
                inner.waker = Some(cx.waker().clone());
                return Poll::Pending;
            }

            inner.data.extend_from_slice(buf);
            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }
            Poll::Ready(Ok(len))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn noise_handshake() -> anyhow::Result<()> {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let initiator_keys = Arc::new(x25519::KeyPair::new(&mut rng));
        let responder_keys = Arc::new(x25519::KeyPair::new(&mut rng));

        let (initiator_stream, responder_stream) = mock_streams();

        let psk = generate_psk(*responder_keys.public_key(), NoiseVersion::V1)?;
        let pattern = NoisePattern::default();

        let stream_initiator = NoiseStreamBuilder::new(initiator_stream)
            .perform_initiator_handshake_inner(
                pattern,
                initiator_keys.private_key().to_bytes(),
                responder_keys.public_key().to_bytes(),
                psk,
                NoiseVersion::V1,
            );

        let stream_responder = NoiseStreamBuilder::new(responder_stream)
            .perform_responder_handshake_inner(
                pattern,
                responder_keys.private_key().to_bytes(),
                *responder_keys.public_key(),
            );

        let initiator_fut =
            tokio::spawn(
                async move { timeout(Duration::from_millis(200), stream_initiator).await },
            );
        let responder_fut =
            tokio::spawn(
                async move { timeout(Duration::from_millis(200), stream_responder).await },
            );

        let (initiator, responder) = join!(initiator_fut, responder_fut);

        let mut initiator = initiator???;
        let mut responder = responder???;

        let msg = b"hello there";
        // if noise was successful we should be able to write a proper message across
        timeout(Duration::from_millis(200), initiator.write_all(msg)).await??;

        initiator.inner_stream.flush().await?;

        let inner_buf = initiator.inner_stream.get_ref().unchecked_tx_data();

        let mut buf = [0u8; 11];
        timeout(Duration::from_millis(200), responder.read(&mut buf)).await??;

        assert_eq!(&buf[..], msg);

        // the inner content is different from the actual msg since it was encrypted
        assert_ne!(inner_buf, buf);
        assert_ne!(inner_buf.len(), msg.len());

        Ok(())
    }
}
