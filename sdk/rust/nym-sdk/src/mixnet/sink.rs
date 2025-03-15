// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::BytesMut;
use futures::{ready, SinkExt};
use nym_client_core::client::inbound_messages::InputMessage;
use tokio::{io::AsyncWrite, sync::mpsc, task::JoinHandle};
use tokio_util::sync::PollSender;

use crate::Error;

use super::MixnetMessageSender;

// The size of the buffer used to send messages to the mixnet. This is used to signal backpressure
// to the caller when the buffer is full. The size is denominated in number of messages, not bytes.
const SINK_BUFFER_SIZE_IN_MESSAGES: usize = 8;

// Traits that represents the ability to convert bytes into InputMessages that can be sent to the
// mixnet. This is typically used to set the destination and other sending parameters.
pub trait BytesToInputMessage: Unpin {
    fn to_input_message(&self, bytes: bytes::Bytes) -> Result<InputMessage, Error>;
}

// Wrapper around MixnetMessageSender that implements AsyncWrite and takes bytes and sends them as
// InputMessages to the mixnet. This requires a BytesToInputMessage implementation to convert bytes
// to InputMessages, which typically means setting the destination and other sending parameters.
pub struct MixnetMessageSink<F>
where
    F: BytesToInputMessage,
{
    packet_translator: F,

    tx: PollSender<InputMessage>,
    send_task: JoinHandle<()>,
}

impl<F> MixnetMessageSink<F>
where
    F: BytesToInputMessage,
{
    pub fn new<Sender>(mixnet_client_sender: Sender, packet_translator: F) -> Self
    where
        Sender: MixnetMessageSender + Send + 'static,
    {
        let (tx, send_task) = Self::start_sender_task(mixnet_client_sender);

        MixnetMessageSink {
            packet_translator,
            tx: PollSender::new(tx),
            send_task,
        }
    }

    fn start_sender_task<Sender>(
        mixnet_client_sender: Sender,
    ) -> (mpsc::Sender<InputMessage>, JoinHandle<()>)
    where
        Sender: MixnetMessageSender + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel(SINK_BUFFER_SIZE_IN_MESSAGES);

        let send_task = tokio::spawn(async move {
            while let Some(input_message) = rx.recv().await {
                if let Err(err) = mixnet_client_sender.send(input_message).await {
                    log::error!("failed to send packet to mixnet: {err}");
                }
            }
        });

        (tx, send_task)
    }
}

impl<F> Drop for MixnetMessageSink<F>
where
    F: BytesToInputMessage,
{
    fn drop(&mut self) {
        self.send_task.abort();
    }
}

impl<F> AsyncWrite for MixnetMessageSink<F>
where
    F: BytesToInputMessage,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        ready!(self.tx.poll_ready_unpin(cx)).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;

        let packet = BytesMut::from(buf).freeze();
        let input_message = self
            .packet_translator
            .to_input_message(packet)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

        // Pass it to the mixnet sender
        self.tx.start_send_unpin(input_message).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        ready!(self.tx.poll_flush_unpin(cx)).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}
