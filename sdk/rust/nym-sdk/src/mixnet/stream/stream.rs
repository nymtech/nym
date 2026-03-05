//! Per-stream handle implementing `AsyncRead + AsyncWrite`.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use futures::{ready, SinkExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

use nym_client_core::client::base_client::ClientInput;
use nym_client_core::client::inbound_messages::InputMessage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;

use super::protocol::{encode_stream_message, StreamId, StreamMessageType};
use super::StreamMap;

/// How to address outbound messages on this stream.
enum Destination {
    /// We know the peer's Nym address.
    Address {
        recipient: Recipient,
        reply_surbs: u32,
    },
    /// We reply via the opener's anonymous sender tag.
    Anonymous { sender_tag: AnonymousSenderTag },
}

/// A byte stream to a single remote Nym client.
///
/// Provides `AsyncRead + AsyncWrite`. Created via
/// [`MixnetClient::open_stream`] (outbound) or
/// [`MixnetListener::accept`] (inbound).
pub struct MixnetStream {
    id: StreamId,
    destination: Destination,
    client_input: ClientInput,
    packet_type: Option<PacketType>,
    streams: StreamMap,

    inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    read_buf: BytesMut,
    deregistered: bool,
}

impl MixnetStream {
    /// Create a stream we initiated to a known recipient.
    pub(crate) fn new_outbound(
        id: StreamId,
        recipient: Recipient,
        reply_surbs: u32,
        client_input: ClientInput,
        packet_type: Option<PacketType>,
        streams: StreamMap,
        inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Self {
        Self {
            id,
            destination: Destination::Address {
                recipient,
                reply_surbs,
            },
            client_input,
            packet_type,
            streams,
            inbound_rx,
            read_buf: BytesMut::new(),
            deregistered: false,
        }
    }

    /// Create a stream accepted from a remote peer's Open message.
    pub(crate) fn new_inbound(
        id: StreamId,
        sender_tag: AnonymousSenderTag,
        client_input: ClientInput,
        packet_type: Option<PacketType>,
        streams: StreamMap,
        inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        initial_data: Vec<u8>,
    ) -> Self {
        let mut read_buf = BytesMut::new();
        if !initial_data.is_empty() {
            read_buf.extend_from_slice(&initial_data);
        }
        Self {
            id,
            destination: Destination::Anonymous { sender_tag },
            client_input,
            packet_type,
            streams,
            inbound_rx,
            read_buf,
            deregistered: false,
        }
    }

    /// Return the unique identifier for this stream.
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Wrap `data` in the appropriate `InputMessage` for this stream's destination.
    fn make_input_message(&self, data: Vec<u8>) -> InputMessage {
        match &self.destination {
            Destination::Address {
                recipient,
                reply_surbs,
            } => InputMessage::new_anonymous(
                *recipient,
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
            self.streams
                .lock()
                .expect("stream map poisoned")
                .remove(&self.id);
        }
    }
}

impl AsyncRead for MixnetStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<std::io::Result<()>> {
        // Drain spillover first
        if !self.read_buf.is_empty() {
            let n = std::cmp::min(buf.remaining(), self.read_buf.len());
            buf.put_slice(&self.read_buf.split_to(n));
            return Poll::Ready(Ok(()));
        }

        match ready!(self.inbound_rx.poll_recv(cx)) {
            Some(data) => {
                let n = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..n]);
                if n < data.len() {
                    self.read_buf.extend_from_slice(&data[n..]);
                }
                Poll::Ready(Ok(()))
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
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        ready!(self.client_input.input_sender.poll_ready_unpin(cx))
            .map_err(|_| std::io::Error::other("mixnet input channel closed"))?;

        let wire = encode_stream_message(&self.id, StreamMessageType::Data, buf);
        let msg = self.make_input_message(wire);

        self.client_input
            .input_sender
            .start_send_unpin(msg)
            .map_err(|_| std::io::Error::other("failed to send stream message"))?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.deregistered {
            self.streams
                .lock()
                .expect("stream map poisoned")
                .remove(&self.id);
            self.deregistered = true;
        }
        Poll::Ready(Ok(()))
    }
}
