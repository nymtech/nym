// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::NoisePattern;
use crate::error::NoiseError;
use crate::stream::codec::NymNoiseCodec;
use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};
use pin_project::pin_project;
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

/// Wrapper around a TcpStream
#[pin_project]
pub struct NoiseStream<C> {
    #[pin]
    inner_stream: Framed<C, NymNoiseCodec>,
    handshake: Option<HandshakeState>,
    noise: Option<TransportState>,
    dec_buffer: BytesMut,
}

impl<C> NoiseStream<C> {
    fn new_inner(inner_stream: C, handshake: HandshakeState) -> NoiseStream<C>
    where
        C: AsyncRead + AsyncWrite,
    {
        NoiseStream {
            inner_stream: Framed::new(inner_stream, NymNoiseCodec::new()),
            handshake: Some(handshake),
            noise: None,
            dec_buffer: BytesMut::new(),
        }
    }

    pub(crate) fn new_initiator(
        inner_stream: C,
        pattern: NoisePattern,
        local_private_key: impl AsRef<[u8]>,
        remote_pub_key: impl AsRef<[u8]>,
        psk: &Psk,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let handshake = Builder::new(pattern.as_noise_params())
            .local_private_key(local_private_key.as_ref())
            .remote_public_key(remote_pub_key.as_ref())
            .psk(pattern.psk_position(), psk)
            .build_initiator()?;
        Ok(NoiseStream::new_inner(inner_stream, handshake))
    }

    pub(crate) fn new_responder(
        inner_stream: C,
        pattern: NoisePattern,
        local_private_key: impl AsRef<[u8]>,
        psk: &Psk,
    ) -> Result<NoiseStream<C>, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let handshake = Builder::new(pattern.as_noise_params())
            .local_private_key(local_private_key.as_ref())
            .psk(pattern.psk_position(), psk)
            .build_responder()?;
        Ok(NoiseStream::new_inner(inner_stream, handshake))
    }

    pub(crate) async fn perform_handshake(mut self) -> Result<Self, NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        //Check if we are in the correct state
        let Some(mut handshake) = self.handshake.take() else {
            return Err(NoiseError::IncorrectStateError);
        };

        while !handshake.is_handshake_finished() {
            if handshake.is_my_turn() {
                self.send_handshake_msg(&mut handshake)
                    .await
                    .inspect_err(|err| println!("send failure: {err}"))?;
            } else {
                self.recv_handshake_msg(&mut handshake)
                    .await
                    .inspect_err(|err| println!("receive failure: {err}"))?;
            }
        }

        self.noise = Some(handshake.into_transport_mode()?);
        Ok(self)
    }

    async fn send_handshake_msg(&mut self, handshake: &mut HandshakeState) -> Result<(), NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        let mut buf = BytesMut::zeroed(HANDSHAKE_MAX_LEN); // we're in the handshake, we can afford a smaller buffer
        let len = handshake.write_message(&[], &mut buf)?;
        buf.truncate(len);
        self.inner_stream.send(buf.into()).await?;
        Ok(())
    }

    async fn recv_handshake_msg(&mut self, handshake: &mut HandshakeState) -> Result<(), NoiseError>
    where
        C: AsyncRead + AsyncWrite + Unpin,
    {
        match self.inner_stream.next().await {
            Some(Ok(msg)) => {
                let mut buf = BytesMut::zeroed(HANDSHAKE_MAX_LEN); // we're in the handshake, we can afford a smaller buffer
                handshake.read_message(&msg, &mut buf)?;
                Ok(())
            }
            Some(Err(err)) => Err(NoiseError::IoError(err)),
            None => Err(NoiseError::HandshakeError),
        }
    }
}

impl<C> AsyncRead for NoiseStream<C>
where
    C: AsyncRead,
{
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
                let mut dec_msg = BytesMut::zeroed(noise_msg.len() - TAGLEN);
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

impl<C> AsyncWrite for NoiseStream<C>
where
    C: AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut projected_self = self.project();

        // returns on Poll::Pending and Poll:Ready(Err)
        ready!(projected_self.inner_stream.as_mut().poll_ready(cx))?;

        // Ready to send, encrypting message
        let mut noise_buf = BytesMut::zeroed(buf.len() + TAGLEN);

        let Ok(len) = (match projected_self.noise {
            Some(transport_state) => transport_state.write_message(buf, &mut noise_buf),
            None => return Poll::Ready(Err(io::ErrorKind::Other.into())),
        }) else {
            return Poll::Ready(Err(io::ErrorKind::InvalidInput.into()));
        };
        noise_buf.truncate(len);

        // Tokio uses the same `start_send ` in their SinkWriter implementation. https://docs.rs/tokio-util/latest/src/tokio_util/io/sink_writer.rs.html#104
        match projected_self
            .inner_stream
            .as_mut()
            .start_send(noise_buf.into())
        {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(e) => Poll::Ready(Err(e)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_psk_v1;
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
    async fn noise_naive_handshake() -> anyhow::Result<()> {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let initiator_keys = Arc::new(x25519::KeyPair::new(&mut rng));
        let responder_keys = Arc::new(x25519::KeyPair::new(&mut rng));

        let (initiator_stream, responder_stream) = mock_streams();

        let psk = generate_psk_v1(*responder_keys.public_key());

        let stream_initiator = NoiseStream::new_initiator(
            initiator_stream,
            NoisePattern::default(),
            initiator_keys.private_key(),
            responder_keys.public_key(),
            &psk,
        )?;

        let stream_responder = NoiseStream::new_responder(
            responder_stream,
            NoisePattern::default(),
            responder_keys.private_key(),
            &psk,
        )?;

        let initiator_fut =
            tokio::spawn(async move { stream_initiator.perform_handshake().await.unwrap() });
        let responder_fut =
            tokio::spawn(async move { stream_responder.perform_handshake().await.unwrap() });

        let (initiator, responder) = join!(initiator_fut, responder_fut);

        let mut initiator = initiator?;
        let mut responder = responder?;

        let msg = b"hello there";
        // if noise was successful we should be able to write a proper message across
        timeout(Duration::from_millis(100), initiator.write_all(msg)).await??;

        initiator.inner_stream.flush().await?;

        let inner_buf = initiator
            .inner_stream
            .get_mut()
            .inner
            .tx
            .lock()
            .await
            .data
            .clone();

        let mut buf = [0u8; 11];
        timeout(Duration::from_millis(100), responder.read(&mut buf)).await??;

        assert_eq!(&buf[..], msg);

        // the inner content is different from the actual msg since it was encrypted
        assert_ne!(inner_buf, buf);
        assert_ne!(inner_buf.len(), msg.len());

        Ok(())
    }
}
