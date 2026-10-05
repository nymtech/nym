// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Per-stream handle implementing `AsyncRead + AsyncWrite`.

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::BytesMut;
use futures::{ready, SinkExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, watch};

use nym_client_core::client::base_client::ClientInput;
use nym_client_core::client::inbound_messages::InputMessage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use tokio_util::sync::PollSender;

use nym_lp_data::packet::frame::SphinxStreamMsgType;

use super::protocol::{encode_stream_message, StreamId};
use super::{StreamFailure, StreamMap};

/// How to address outbound messages on this stream.
enum Destination {
    /// We know the peer's Nym address.
    Address {
        recipient: Box<Recipient>,
        reply_surbs: u32,
    },
    /// We reply via the dialer's anonymous sender tag.
    Anonymous { sender_tag: AnonymousSenderTag },
}

/// A byte stream to a single remote Nym client.
///
/// Provides `AsyncRead + AsyncWrite`. Created via
/// [`MixnetClient::open_stream`](crate::mixnet::MixnetClient::open_stream) (outbound) or
/// [`MixnetListener::accept`](super::MixnetListener::accept) (inbound).
pub struct MixnetStream {
    id: StreamId,
    destination: Destination,
    sender: PollSender<InputMessage>,
    packet_type: Option<PacketType>,
    streams: StreamMap,

    inbound_rx: mpsc::UnboundedReceiver<Result<Vec<u8>, StreamFailure>>,
    read_buf: BytesMut,
    deregistered: bool,
    next_seq: u32,

    /// Set when `poll_read` hits a lost-data marker. Once set, all reads
    /// and writes fail. `recv()` never sets this: it returns the error
    /// once and keeps going.
    failure: Option<StreamFailure>,

    /// Flips to true when the peer acknowledges the stream. Already true
    /// for inbound streams: we accepted them ourselves.
    established_rx: watch::Receiver<bool>,
}

impl MixnetStream {
    /// Create a stream we initiated to a known recipient.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_outbound(
        id: StreamId,
        recipient: Recipient,
        reply_surbs: u32,
        client_input: ClientInput,
        packet_type: Option<PacketType>,
        streams: StreamMap,
        inbound_rx: mpsc::UnboundedReceiver<Result<Vec<u8>, StreamFailure>>,
        established_rx: watch::Receiver<bool>,
    ) -> Self {
        let sender = PollSender::new(client_input.input_sender.clone());
        Self {
            id,
            destination: Destination::Address {
                recipient: Box::new(recipient),
                reply_surbs,
            },
            sender,
            packet_type,
            streams,
            inbound_rx,
            read_buf: BytesMut::new(),
            deregistered: false,
            next_seq: 0,
            failure: None,
            established_rx,
        }
    }

    /// Create a stream accepted from a remote peer's Open message.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_inbound(
        id: StreamId,
        sender_tag: AnonymousSenderTag,
        client_input: ClientInput,
        packet_type: Option<PacketType>,
        streams: StreamMap,
        inbound_rx: mpsc::UnboundedReceiver<Result<Vec<u8>, StreamFailure>>,
        established_rx: watch::Receiver<bool>,
        initial_data: Vec<u8>,
    ) -> Self {
        let mut read_buf = BytesMut::new();
        if !initial_data.is_empty() {
            read_buf.extend_from_slice(&initial_data);
        }
        let sender = PollSender::new(client_input.input_sender.clone());
        Self {
            id,
            destination: Destination::Anonymous { sender_tag },
            sender,
            packet_type,
            streams,
            inbound_rx,
            read_buf,
            deregistered: false,
            next_seq: 0,
            failure: None,
            established_rx,
        }
    }

    /// Return the unique identifier for this stream.
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Wait up to `timeout` for the peer to acknowledge the stream.
    ///
    /// Resolves once the peer's `OpenAck` arrives, or once the peer sends
    /// data (which proves it accepted the stream, covering a lost ack).
    /// On an inbound stream it resolves immediately.
    ///
    /// A timeout means the peer's state is unknown: it may run an SDK
    /// without establishment support, have no reply SURBs left, or be
    /// gone. The stream stays usable after a timeout, so this is not a
    /// reason to discard it.
    ///
    /// Pick the timeout to suit the caller: a mixnet round trip is
    /// seconds, and a cold establishment can take appreciably longer.
    pub async fn wait_established(&mut self, timeout: Duration) -> std::io::Result<()> {
        match tokio::time::timeout(timeout, self.established_rx.wait_for(|v| *v)).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => Err(std::io::Error::other("stream closed before establishment")),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "no establishment acknowledgement within timeout (peer state unknown)",
            )),
        }
    }

    /// Instant of the most recent inbound frame from the peer, or `None`
    /// once the stream is no longer registered. Reads local state only;
    /// sends nothing.
    pub async fn last_peer_activity(&self) -> Option<tokio::time::Instant> {
        self.streams.last_activity(&self.id).await
    }

    /// Receive a single message payload directly from the stream channel.
    ///
    /// Returns `None` on EOF (channel closed). Returns `Some(Err(_))`
    /// when messages were lost at this point in the sequence; later calls
    /// return the messages that follow. Drains any leftover from a prior
    /// `AsyncRead` call first.
    pub async fn recv(&mut self) -> Option<std::io::Result<Vec<u8>>> {
        if let Some(failure) = &self.failure {
            return Some(Err(failure.as_io_error()));
        }
        if !self.read_buf.is_empty() {
            return Some(Ok(self.read_buf.split().to_vec()));
        }
        Some(
            self.inbound_rx
                .recv()
                .await?
                .map_err(StreamFailure::as_io_error),
        )
    }

    /// Wrap `data` in the appropriate `InputMessage` for this stream's destination.
    fn make_input_message(&self, data: Vec<u8>) -> InputMessage {
        match &self.destination {
            Destination::Address {
                recipient,
                reply_surbs,
            } => InputMessage::new_anonymous(
                **recipient,
                data,
                *reply_surbs,
                TransmissionLane::General,
                self.packet_type,
            ),
            Destination::Anonymous { sender_tag } => InputMessage::new_reply(
                *sender_tag,
                data,
                TransmissionLane::General,
                self.packet_type,
            ),
        }
    }
}

impl Drop for MixnetStream {
    fn drop(&mut self) {
        if !self.deregistered {
            self.streams.remove_background(self.id);
        }
    }
}

impl AsyncRead for MixnetStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<std::io::Result<()>> {
        if let Some(failure) = &self.failure {
            return Poll::Ready(Err(failure.as_io_error()));
        }

        // Drain spillover first
        if !self.read_buf.is_empty() {
            let n = std::cmp::min(buf.remaining(), self.read_buf.len());
            buf.put_slice(&self.read_buf.split_to(n));
            return Poll::Ready(Ok(()));
        }

        match ready!(self.inbound_rx.poll_recv(cx)) {
            Some(Ok(data)) => {
                let n = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..n]);
                if n < data.len() {
                    self.read_buf.extend_from_slice(&data[n..]);
                }
                Poll::Ready(Ok(()))
            }
            Some(Err(failure)) => {
                self.failure = Some(failure);
                Poll::Ready(Err(failure.as_io_error()))
            }
            None => Poll::Ready(Ok(())), // EOF
        }
    }
}

impl AsyncWrite for MixnetStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if let Some(failure) = &self.failure {
            return Poll::Ready(Err(failure.as_io_error()));
        }

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        ready!(self.sender.poll_ready_unpin(cx))
            .map_err(|_| std::io::Error::other("mixnet input channel closed"))?;

        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let wire = encode_stream_message(&self.id, SphinxStreamMsgType::Data, seq, buf);
        let msg = self.make_input_message(wire);

        self.sender
            .start_send_unpin(msg)
            .map_err(|_| std::io::Error::other("failed to send stream message"))?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.deregistered {
            self.streams.remove_background(self.id);
            self.deregistered = true;
        }
        Poll::Ready(Ok(()))
    }
}
