// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet::{AnonymousSenderTag, IncludedSurbs, Recipient};
use crate::{Error, Result};
use async_trait::async_trait;
use nym_client_core::client::inbound_messages::InputMessage;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;

/// Trait for sending messages through the Nym mixnet.
///
/// Implemented by both [`MixnetClient`](crate::mixnet::MixnetClient) and
/// [`MixnetClientSender`](crate::mixnet::MixnetClientSender), allowing code
/// to be generic over the sender type.
#[async_trait]
pub trait MixnetMessageSender {
    fn packet_type(&self) -> Option<PacketType> {
        None
    }

    /// Sends a [`InputMessage`] to the mixnet. This is the most low-level sending function, for
    /// full customization.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. The message is either fully queued or not
    /// sent at all.
    async fn send(&self, message: InputMessage) -> Result<()>;

    /// Sends a [`InputMessage`] over the Lewes Protocol rather than the gateway websocket.
    ///
    /// Only [`InputMessage::Regular`] travels this way. The LP path carries no reply-SURBs, so an
    /// anonymous or reply message is dropped by the data handler rather than sent - use
    /// [`send_plain_message_over_lp`](Self::send_plain_message_over_lp), which builds the right
    /// kind.
    ///
    /// A client with an LP path accepts the message whether or not it holds a session: one with no
    /// session to travel on is dropped by the data plane rather than refused here. Whether the
    /// session was established is said once, at startup.
    ///
    /// # Errors
    ///
    /// [`Error::NoLpSession`] from a sender that has no LP path at all - the default below.
    async fn send_lp(&self, message: InputMessage) -> Result<()> {
        let _ = message;
        Err(Error::NoLpSession)
    }

    /// Sends data over the Lewes Protocol to the supplied Nym address, exposing our own address.
    ///
    /// The counterpart of [`send_plain_message`](Self::send_plain_message), which defaults to
    /// carrying reply-SURBs that the LP path has no room for.
    async fn send_plain_message_over_lp<M>(&self, address: Recipient, message: M) -> Result<()>
    where
        M: AsRef<[u8]> + Send,
    {
        self.send_lp(InputMessage::new_regular(
            address,
            message.as_ref().to_vec(),
            TransmissionLane::General,
            self.packet_type(),
        ))
        .await
    }

    /// Sends data to the supplied Nym address with the default surb behaviour.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{self, MixnetMessageSender};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    /// let addr = *client.nym_address();
    ///
    /// client.send_plain_message(addr, "hello").await.unwrap();
    /// # }
    /// ```
    async fn send_plain_message<M>(&self, address: Recipient, message: M) -> Result<()>
    where
        M: AsRef<[u8]> + Send,
    {
        self.send_message(address, message, IncludedSurbs::default())
            .await
    }

    /// Sends bytes to the supplied Nym address. There is the option to specify the number of
    /// reply-SURBs to include.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{self, MixnetMessageSender};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    /// let addr = *client.nym_address();
    /// let surbs = mixnet::IncludedSurbs::new(5);
    ///
    /// client.send_message(addr, b"hello", surbs).await.unwrap();
    /// # }
    /// ```
    async fn send_message<M>(
        &self,
        address: Recipient,
        message: M,
        surbs: IncludedSurbs,
    ) -> Result<()>
    where
        M: AsRef<[u8]> + Send,
    {
        let lane = TransmissionLane::General;
        let input_msg = match surbs {
            IncludedSurbs::Amount(surbs) => InputMessage::new_anonymous(
                address,
                message.as_ref().to_vec(),
                surbs,
                lane,
                self.packet_type(),
            ),
            IncludedSurbs::ExposeSelfAddress => InputMessage::new_regular(
                address,
                message.as_ref().to_vec(),
                lane,
                self.packet_type(),
            ),
        };
        self.send(input_msg).await
    }

    /// Sends reply data to the supplied anonymous recipient.
    ///
    /// The [`AnonymousSenderTag`] comes from a received message's
    /// [`sender_tag`](nym_sphinx::receiver::ReconstructedMessage::sender_tag) field.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{self, MixnetMessageSender};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///
    /// if let Some(msgs) = client.wait_for_messages().await {
    ///     for msg in msgs {
    ///         if let Some(tag) = msg.sender_tag {
    ///             client.send_reply(tag, b"got it!").await.unwrap();
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    async fn send_reply<M>(&self, recipient_tag: AnonymousSenderTag, message: M) -> Result<()>
    where
        M: AsRef<[u8]> + Send,
    {
        let lane = TransmissionLane::General;
        let input_msg = InputMessage::new_reply(
            recipient_tag,
            message.as_ref().to_vec(),
            lane,
            self.packet_type(),
        );
        self.send(input_msg).await
    }
}
