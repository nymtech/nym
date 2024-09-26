// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::messages::{Finalization, GatewayMaterialExchange};
use crate::registration::handshake::state::State;
use crate::registration::handshake::SharedGatewayKey;
use crate::registration::handshake::{error::HandshakeError, WsItem};
use futures::{Sink, Stream};
use rand::{CryptoRng, RngCore};
use tungstenite::Message as WsMessage;

impl<'a, S, R> State<'a, S, R> {
    async fn client_handshake_inner(&mut self) -> Result<(), HandshakeError>
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
        R: CryptoRng + RngCore,
    {
        // 1. if we're using non-legacy, i.e. aes256gcm-siv derivation, generate initiator salt for kdf
        let maybe_hkdf_salt = self.maybe_generate_initiator_salt();

        // 1. send ed25519 pubkey alongside ephemeral x25519 pubkey and a hkdf salt if we're using non-legacy client
        // LOCAL_ID_PUBKEY || EPHEMERAL_KEY || MAYBE_SALT
        let init_message = self.init_message(maybe_hkdf_salt.clone());
        self.send_handshake_data(init_message).await?;

        // 2. wait for response with remote x25519 pubkey as well as encrypted signature
        // <- g^y || AES(k, sig(gate_priv, (g^y || g^x)) || MAYBE_NONCE
        let mid_res = self
            .receive_handshake_message::<GatewayMaterialExchange>()
            .await?;

        // 3. derive shared keys locally
        // hkdf::<blake3>::(g^xy)
        self.derive_shared_key(&mid_res.ephemeral_dh, maybe_hkdf_salt.as_deref());

        // 4. verify the received signature using the locally derived keys
        self.verify_remote_key_material(&mid_res.materials, &mid_res.ephemeral_dh)?;

        // 5. produce our own materials to get verified by the remote
        // -> AES(k, sig(client_priv, g^x || g^y)) || MAYBE_NONCE
        let materials = self.prepare_key_material_sig(&mid_res.ephemeral_dh)?;
        self.send_handshake_data(materials).await?;

        // 6. wait for remote confirmation of finalizing the handshake
        let finalization = self.receive_handshake_message::<Finalization>().await?;
        finalization.ensure_success()?;
        Ok(())
    }

    pub(crate) async fn perform_client_handshake(
        mut self,
    ) -> Result<SharedGatewayKey, HandshakeError>
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
        R: CryptoRng + RngCore,
    {
        let handshake_res = self.client_handshake_inner().await;
        self.check_for_handshake_processing_error(handshake_res)
            .await?;
        Ok(self.finalize_handshake())
    }
}
