// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! Substream implementation providing AsyncRead + AsyncWrite over Nym mixnet.

use super::message::{
    ConnectionId, Message, OutboundMessage, SubstreamId, SubstreamMessage, TransportMessage,
};
use futures::{
    channel::{mpsc::UnboundedReceiver, oneshot::Receiver},
    io::{Error as IoError, ErrorKind},
    AsyncRead, AsyncWrite, FutureExt, StreamExt,
};
use log::debug;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use parking_lot::Mutex;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

// Re-export UnboundedSender for use in other modules
pub(crate) use futures::channel::mpsc::UnboundedSender;

#[derive(Debug)]
pub struct Substream {
    remote_recipient: Option<Recipient>,
    connection_id: ConnectionId,
    pub(crate) substream_id: SubstreamId,

    /// inbound messages; inbound_tx is in the corresponding Connection
    pub(crate) inbound_rx: UnboundedReceiver<Vec<u8>>,

    /// outbound messages; go directly to the mixnet
    outbound_tx: UnboundedSender<OutboundMessage>,

    sender_tag: Option<AnonymousSenderTag>,

    /// used to signal when the substream is closed
    close_rx: Receiver<()>,
    closed: Mutex<bool>,

    // buffer of data that's been written to the stream,
    // but not yet read by the application.
    unread_data: Mutex<Vec<u8>>,

    message_nonce: Arc<AtomicU64>,
}

impl Substream {
    pub(crate) fn new_with_sender_tag(
        remote_recipient: Option<Recipient>,
        connection_id: ConnectionId,
        substream_id: SubstreamId,
        inbound_rx: UnboundedReceiver<Vec<u8>>,
        outbound_tx: UnboundedSender<OutboundMessage>,
        close_rx: Receiver<()>,
        message_nonce: Arc<AtomicU64>,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Self {
        Substream {
            remote_recipient,
            connection_id,
            substream_id,
            inbound_rx,
            outbound_tx,
            sender_tag,
            close_rx,
            closed: Mutex::new(false),
            unread_data: Mutex::new(vec![]),
            message_nonce,
        }
    }

    pub(crate) fn new(
        remote_recipient: Option<Recipient>,
        connection_id: ConnectionId,
        substream_id: SubstreamId,
        inbound_rx: UnboundedReceiver<Vec<u8>>,
        outbound_tx: UnboundedSender<OutboundMessage>,
        close_rx: Receiver<()>,
        message_nonce: Arc<AtomicU64>,
    ) -> Self {
        Self::new_with_sender_tag(
            remote_recipient,
            connection_id,
            substream_id,
            inbound_rx,
            outbound_tx,
            close_rx,
            message_nonce,
            None,
        )
    }

    fn check_closed(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Result<(), IoError> {
        let closed_err = IoError::new(ErrorKind::Other, "stream closed");

        // Poll the close receiver to check if close was signaled
        let received_closed = self.close_rx.poll_unpin(cx);

        let mut closed = self.closed.lock();
        if *closed {
            return Err(closed_err);
        }

        if let Poll::Ready(Ok(())) = received_closed {
            *closed = true;
            return Err(closed_err);
        }

        Ok(())
    }
}

impl AsyncRead for Substream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, IoError>> {
        let closed_result = self.as_mut().check_closed(cx);
        if let Err(e) = closed_result {
            return Poll::Ready(Err(e));
        }

        let inbound_rx_data = self.inbound_rx.poll_next_unpin(cx);

        // first, write any previously unread data to the buf
        let mut unread_data = self.unread_data.lock();
        let filled_len = if !unread_data.is_empty() {
            let unread_len = unread_data.len();
            let buf_len = buf.len();
            let copy_len = std::cmp::min(unread_len, buf_len);
            buf[..copy_len].copy_from_slice(&unread_data[..copy_len]);
            *unread_data = unread_data[copy_len..].to_vec();
            copy_len
        } else {
            0
        };

        if let Poll::Ready(Some(data)) = inbound_rx_data {
            if filled_len == buf.len() {
                // we've filled the buffer, so we'll have to save the rest for later
                let mut new = vec![];
                new.extend(unread_data.drain(..));
                new.extend(data.iter());
                *unread_data = new;
                return Poll::Ready(Ok(filled_len));
            }

            // otherwise, there's still room in the buffer, so we'll copy the rest of the data
            let remaining_len = buf.len() - filled_len;
            let data_len = data.len();

            // we have more data than buffer room remaining, save the extra for later
            if remaining_len < data_len {
                unread_data.extend_from_slice(&data[remaining_len..]);
            }

            let copied = std::cmp::min(remaining_len, data_len);
            buf[filled_len..filled_len + copied].copy_from_slice(&data[..copied]);
            debug!("poll_read copied {} bytes", copied);
            return Poll::Ready(Ok(copied));
        }

        if filled_len > 0 {
            debug!("poll_read copied {} bytes", filled_len);
            return Poll::Ready(Ok(filled_len));
        }

        Poll::Pending
    }
}

impl AsyncWrite for Substream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, IoError>> {
        if let Err(e) = self.as_mut().check_closed(cx) {
            return Poll::Ready(Err(e));
        }

        let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);

        self.outbound_tx
            .unbounded_send(OutboundMessage {
                recipient: self.remote_recipient,
                message: Message::TransportMessage(TransportMessage {
                    nonce,
                    id: self.connection_id.clone(),
                    message: SubstreamMessage::new_with_data(
                        self.substream_id.clone(),
                        buf.to_vec(),
                    ),
                }),
                sender_tag: self.sender_tag.clone(),
            })
            .map_err(|e| {
                IoError::new(
                    ErrorKind::Other,
                    format!("poll_write outbound_tx error: {}", e),
                )
            })?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);

        let mut closed = self.closed.lock();
        if *closed {
            return Poll::Ready(Err(IoError::new(ErrorKind::Other, "stream closed")));
        }

        *closed = true;

        // send a close message to the mixnet
        self.outbound_tx
            .unbounded_send(OutboundMessage {
                recipient: self.remote_recipient,
                message: Message::TransportMessage(TransportMessage {
                    nonce,
                    id: self.connection_id.clone(),
                    message: SubstreamMessage::new_close(self.substream_id.clone()),
                }),
                sender_tag: self.sender_tag.clone(),
            })
            .map_err(|e| {
                IoError::new(
                    ErrorKind::Other,
                    format!("poll_close outbound_rx error: {}", e),
                )
            })?;

        Poll::Ready(Ok(()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        if let Err(e) = self.check_closed(cx) {
            return Poll::Ready(Err(e));
        }

        Poll::Ready(Ok(()))
    }
}
