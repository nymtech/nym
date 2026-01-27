// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::messages::{
    HandshakeMessage, Initialisation, MaterialExchange,
};
use crate::registration::handshake::state::State;
use crate::registration::handshake::HandshakeResult;
use crate::registration::handshake::{error::HandshakeError, WsItem};
use crate::{GatewayProtocolVersion, GatewayProtocolVersionExt};
use futures::{Sink, Stream};
use tracing::{debug, warn};
use tungstenite::Message as WsMessage;

impl<S, R> State<'_, S, R> {
    async fn gateway_handshake_inner(
        &mut self,
        raw_init_message: Vec<u8>,
    ) -> Result<(), HandshakeError>
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
    {
        // NEGOTIATE PROTOCOL
        // old clients were sending protocol version as defined by the following:
        /*
          fn request_protocol_version(&self) -> u8 {
               if self.derive_aes256_gcm_siv_key {
                   AES_GCM_SIV_PROTOCOL_VERSION
               } else if self.expects_credential_usage {
                   CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION
               } else {
                   INITIAL_PROTOCOL_VERSION
               }
           }
        */
        // meaning the highest possible value they could have sent was `4` (AUTHENTICATE_V2_PROTOCOL_VERSION)
        // so if we received anything higher than that, it means they understand negotiation.
        // currently not strictly needed as we just blindly accept what they proposed,
        // but will be needed in the future.
        if self.proposed_protocol_version().is_future_version() {
            // this should never happen in a non-malicious client as it should use at most whatever version this gateway has announced
            self.set_protocol_version(GatewayProtocolVersion::CURRENT)
        } else {
            // currently we accept all protocols, i.e. legacy keys, aes128, etc. so we downgrade to whatever
            // the client has proposed. this will change in the future
            debug!(
                "using the protocol version proposed by the client: {:?}",
                self.proposed_protocol_version()
            )
        }

        // 1. receive remote ed25519 pubkey alongside ephemeral x25519 pubkey and maybe a flag indicating non-legacy client
        // LOCAL_ID_PUBKEY || EPHEMERAL_KEY || MAYBE_NON_LEGACY
        let init_message = Initialisation::try_from_bytes(&raw_init_message)?;
        self.update_remote_identity(init_message.identity);

        // 2. derive shared keys locally
        // hkdf::<blake3>::(g^xy)
        self.derive_shared_key(&init_message.ephemeral_dh, &init_message.initiator_salt);

        // 3. send ephemeral x25519 pubkey alongside the encrypted signature
        // g^y || AES(k, sig(gate_priv, (g^y || g^x))
        let material = self
            .prepare_key_material_sig(&init_message.ephemeral_dh)?
            .attach_ephemeral_dh(*self.local_ephemeral_key());
        self.send_handshake_data(material).await?;

        // 4. wait for the remote response with their own encrypted signature
        let (materials, client_protocol) =
            self.receive_handshake_message::<MaterialExchange>().await?;
        if client_protocol != self.proposed_protocol_version() {
            warn!("the client hasn't accepted our proposed protocol version. we suggested {:?} while it returned {client_protocol:?}", self.proposed_protocol_version());
            // TBD what to do here
        }

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
    ) -> Result<HandshakeResult, HandshakeError>
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin,
    {
        let handshake_res = self.gateway_handshake_inner(raw_init_message).await;
        self.check_for_handshake_processing_error(handshake_res)
            .await?;
        Ok(self.finalize_handshake())
    }
}
