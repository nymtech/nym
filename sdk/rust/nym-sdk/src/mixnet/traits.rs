// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet::{AnonymousSenderTag, IncludedSurbs, Recipient};
use crate::Result;
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
