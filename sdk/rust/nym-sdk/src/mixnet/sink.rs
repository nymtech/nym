// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{ready, SinkExt};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_sphinx::{
    addressing::Recipient, anonymous_replies::requests::AnonymousSenderTag, params::PacketType,
};
use nym_task::connections::TransmissionLane;
use tokio::{io::AsyncWrite, sync::mpsc, task::JoinHandle};
use tokio_util::sync::PollSender;

use crate::Error;

use super::{IncludedSurbs, MixnetMessageSender};

// The size of the buffer used to send messages to the mixnet. This is used to signal backpressure
// to the caller when the buffer is full. The size is denominated in number of messages, not bytes.
const SINK_BUFFER_SIZE_IN_MESSAGES: usize = 8;

/// Traits that represents the ability to convert bytes into InputMessages that can be sent to the
/// mixnet. This is typically used to set the destination and other sending parameters.
pub trait MixnetMessageSinkTranslator: Unpin {
    fn to_input_message(&self, bytes: &[u8]) -> Result<InputMessage, Error>;
}

/// The default implementation of MixnetMessageSinkTranslator that sends messages to a recipient or
/// replies to a sender that has provided reply SURBs.
#[derive(Clone, Debug)]
pub struct DefaultMixnetMessageSinkTranslator {
    destination: SinkDestination,
    lane: TransmissionLane,
    packet_type: Option<PacketType>,
}

/// The destination for messages feed to the mixnnet sink, used by the default implementation of
/// MixnetMessageSinkTranslator.
#[derive(Clone, Debug)]
enum SinkDestination {
    Recipient {
        recipient: Box<Recipient>,
        surbs: IncludedSurbs,
    },
    Reply(AnonymousSenderTag),
}

impl MixnetMessageSinkTranslator for DefaultMixnetMessageSinkTranslator {
    fn to_input_message(&self, bytes: &[u8]) -> Result<InputMessage, Error> {
        let bytes = bytes.to_vec();
        match &self.destination {
            SinkDestination::Recipient { recipient, surbs } => match surbs {
                IncludedSurbs::ExposeSelfAddress => Ok(InputMessage::new_regular(
                    **recipient,
                    bytes,
                    self.lane,
                    self.packet_type,
                )),
                IncludedSurbs::Amount(surbs) => Ok(InputMessage::new_anonymous(
                    **recipient,
                    bytes,
                    *surbs,
                    self.lane,
                    self.packet_type,
                )),
            },
            SinkDestination::Reply(tag) => Ok(InputMessage::new_reply(
                *tag,
                bytes,
                self.lane,
                self.packet_type,
            )),
        }
    }
}

/// Wrapper around MixnetMessageSender that implements AsyncWrite and takes bytes and sends them as
/// InputMessages to the mixnet. This requires a BytesToInputMessage implementation to convert bytes
/// to InputMessages, which typically means setting the destination and other sending parameters.
pub struct MixnetMessageSink<F>
where
    F: MixnetMessageSinkTranslator,
{
    // The function that converts bytes into InputMessages
    message_translator: F,

    // Send messages to the mixnet sender task
    tx: PollSender<InputMessage>,

    // The handle for the mixnet sender task
    send_task: JoinHandle<()>,
}

impl MixnetMessageSink<DefaultMixnetMessageSinkTranslator> {
    /// Creates a new MixnetMessageSink that sends messages to the provided recipient. The messages
    /// can also include SURBs to allow for anonymous communication.
    ///
    /// Typically you don't want to include SURBs here, but instead provide an initial set of SURBs
    /// on the first message, and then let the recipient request more SURBs as needed.
    ///
    /// If you provide SURBs here, the recipient will very likely receive far more SURBs than they
    /// need.
    pub fn new_recipient_sink<Sender>(
        mixnet_client_sender: Sender,
        recipient: Recipient,
        surbs: IncludedSurbs,
    ) -> Self
    where
        Sender: MixnetMessageSender + Send + 'static,
    {
        let destination = SinkDestination::Recipient {
            recipient: Box::new(recipient),
            surbs,
        };
        let translator = DefaultMixnetMessageSinkTranslator {
            destination,
            lane: TransmissionLane::General,
            packet_type: None,
        };
        Self::new_with_custom_translator(mixnet_client_sender, translator)
    }

    /// Creates a new MixnetMessageSink that sends messages to a recipient with the provided SURBs.
    /// The messages are sent using the provided MixnetMessageSender.
    pub fn new_reply_sink<Sender>(
        mixnet_client_sender: Sender,
        recipient_tag: AnonymousSenderTag,
    ) -> Self
    where
        Sender: MixnetMessageSender + Send + 'static,
    {
        let destination = SinkDestination::Reply(recipient_tag);
        let translator = DefaultMixnetMessageSinkTranslator {
            destination,
            lane: TransmissionLane::General,
            packet_type: None,
        };
        Self::new_with_custom_translator(mixnet_client_sender, translator)
    }
}

impl<F> MixnetMessageSink<F>
where
    F: MixnetMessageSinkTranslator,
{
    /// Creates a new MixnetMessageSink that sends messages to the mixnet using the provided
    /// MixnetMessageSender. The messages are converted to InputMessages using the provided
    /// MixnetMessageSinkTranslator.
    ///
    /// The usecase of this function is to allow for custom message translation, for example to
    /// wrap messages in a specific way or to add additional metadata.
    pub fn new_with_custom_translator<Sender>(
        mixnet_client_sender: Sender,
        message_translator: F,
    ) -> Self
    where
        Sender: MixnetMessageSender + Send + 'static,
    {
        // Create a separate task to send messages to the mixnet. This is driven mostly by the
        // implementation of AsyncWrite.
        let (tx, send_task) = Self::start_sender_task(mixnet_client_sender);

        // Wrap the sender in PollSener to make the AsyncWrite implementation more ergonomic
        let tx = PollSender::new(tx);

        MixnetMessageSink {
            message_translator,
            tx,
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
    F: MixnetMessageSinkTranslator,
{
    fn drop(&mut self) {
        self.send_task.abort();
    }
}

impl<F> AsyncWrite for MixnetMessageSink<F>
where
    F: MixnetMessageSinkTranslator,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        ready!(self.tx.poll_ready_unpin(cx))
            .map_err(|_| std::io::Error::other("failed to send packet to mixnet"))?;

        let input_message = self
            .message_translator
            .to_input_message(buf)
            .map_err(std::io::Error::other)?;

        // Pass it to the mixnet sender
        self.tx
            .start_send_unpin(input_message)
            .map_err(|_| std::io::Error::other("failed to send packet to mixnet"))?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        ready!(self.tx.poll_flush_unpin(cx))
            .map_err(|_| std::io::Error::other("failed to send packet to mixnet"))?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}
