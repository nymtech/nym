// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet::{AnonymousSenderTag, IncludedSurbs, Recipient};
use crate::Result;
use async_trait::async_trait;
use nym_client_core::client::inbound_messages::InputMessage;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;

// defined to guarantee common interface regardless of whether you're using the full client
// or just the sending handler
#[async_trait]
pub trait MixnetMessageSender {
    fn packet_type(&self) -> Option<PacketType> {
        None
    }

    /// Sends a [`InputMessage`] to the mixnet. This is the most low-level sending function, for
    /// full customization.
    async fn send(&mut self, message: InputMessage) -> Result<()>;

    /// Sends data to the supplied Nym address with the default surb behaviour.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{self, MixnetMessageSender};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let address = "foobar";
    ///     let recipient = mixnet::Recipient::try_from_base58_string(address).unwrap();
    ///     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     client.send_plain_message(recipient, "hi").await.unwrap();
    /// }
    /// ```
    async fn send_plain_message<M>(&mut self, address: Recipient, message: M) -> Result<()>
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
    /// #[tokio::main]
    /// async fn main() {
    ///     let address = "foobar";
    ///     let recipient = mixnet::Recipient::try_from_base58_string(address).unwrap();
    ///     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     let surbs = mixnet::IncludedSurbs::default();
    ///     client.send_message(recipient, "hi".to_owned().into_bytes(), surbs).await.unwrap();
    /// }
    /// ```
    async fn send_message<M>(
        &mut self,
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
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{self, MixnetMessageSender};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     // note: the tag is something you would have received from a remote client sending you surbs!
    ///     let tag = mixnet::AnonymousSenderTag::try_from_base58_string("foobar").unwrap();
    ///     client.send_reply(tag, b"hi").await.unwrap();
    /// }
    /// ```
    async fn send_reply<M>(&mut self, recipient_tag: AnonymousSenderTag, message: M) -> Result<()>
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
