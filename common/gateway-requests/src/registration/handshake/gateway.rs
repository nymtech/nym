// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::messages::{
    HandshakeMessage, Initialisation, MaterialExchange,
};
use crate::registration::handshake::state::State;
use crate::registration::handshake::SharedGatewayKey;
use crate::registration::handshake::{error::HandshakeError, WsItem};
use futures::{Sink, Stream};
use tungstenite::Message as WsMessage;

impl<'a, S> State<'a, S> {
    async fn gateway_handshake_inner(
        &mut self,
        raw_init_message: Vec<u8>,
    ) -> Result<(), HandshakeError>
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
    {
        // 1. receive remote ed25519 pubkey alongside ephemeral x25519 pubkey and maybe a flag indicating non-legacy client
        // LOCAL_ID_PUBKEY || EPHEMERAL_KEY || MAYBE_NON_LEGACY
        let init_message = Initialisation::try_from_bytes(&raw_init_message)?;
        self.update_remote_identity(init_message.identity);
        self.set_aes256_gcm_siv_key_derivation(init_message.derive_aes256_gcm_siv_key);

        // 2. derive shared keys locally
        // hkdf::<blake3>::(g^xy)
        self.derive_shared_key(&init_message.ephemeral_dh);

        // 3. send ephemeral x25519 pubkey alongside the encrypted signature
        // g^y || AES(k, sig(gate_priv, (g^y || g^x))
        let material = self
            .prepare_key_material_sig(&init_message.ephemeral_dh)?
            .attach_ephemeral_dh(*self.local_ephemeral_key());
        self.send_handshake_data(material).await?;

        // 4. wait for the remote response with their own encrypted signature
        let materials = self.receive_handshake_message::<MaterialExchange>().await?;

        // 5. verify the received signature using the locally derived keys
        self.verify_remote_key_material(&materials, &init_message.ephemeral_dh)?;

        // 6. finally send the finalization message to conclude the exchange
        let finalizer = self.finalization_message();
        self.send_handshake_data(finalizer).await?;

        Ok(())
    }

    pub(crate) async fn perform_gateway_handshake(
        mut self,
        raw_init_message: Vec<u8>,
    ) -> Result<SharedGatewayKey, HandshakeError>
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
    {
        let handshake_res = self.gateway_handshake_inner(raw_init_message).await;
        self.check_for_handshake_processing_error(handshake_res)
            .await?;
        Ok(self.finalize_handshake())
    }
}
