use crypto::asymmetric::encryption::KeyPair;
use futures::StreamExt;
use log::debug;
use nymsphinx::receiver::MessageReceiver;

use super::MixnetReceiver;

pub(crate) struct MixnetListener {
    client_encryption_keypair: KeyPair,
    message_receiver: MessageReceiver,
    mixnet_receiver: MixnetReceiver,
}

impl MixnetListener {
    pub(crate) fn new(
        mixnet_receiver: MixnetReceiver,
        client_encryption_keypair: KeyPair,
    ) -> MixnetListener {
        let message_receiver = MessageReceiver::new();
        MixnetListener {
            client_encryption_keypair,
            message_receiver,
            mixnet_receiver,
        }
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started AcknowledgementListener");
        while let Some(messages) = self.mixnet_receiver.next().await {
            // realistically we would only be getting one ack at the time
            for message in messages {
                self.on_message(message).await;
            }
        }
        panic!("MixnetListener channel failed. This should never happen.")
    }

    async fn on_message(&mut self, message: Vec<u8>) {
        let encrypted_bytes = self
            .message_receiver
            .recover_plaintext(self.client_encryption_keypair.private_key(), message)
            .unwrap();
        let fragment = self
            .message_receiver
            .recover_fragment(&encrypted_bytes)
            .unwrap();
        let (recovered, _) = self
            .message_receiver
            .insert_new_fragment(fragment)
            .unwrap()
            .unwrap();
        let message = String::from_utf8_lossy(&recovered.message);
        println!("got msg: {:?}", message);
    }
}
