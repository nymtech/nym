// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, time::Duration};

use libcrux_ed25519::VerificationKey;
use libcrux_psq::{
    cred::Ed25519,
    psk_registration::{Initiator, InitiatorMsg, Responder, ResponderMsg},
    traits::PSQ,
};

use tls_codec::{Deserialize, Serialize};

use libcrux_traits::kem::KEM;

use rand::{CryptoRng, RngCore};

use crate::error::PSQError;

pub const PSK_LENGTH: usize = 32;
pub const CONTEXT_LEN: usize = 16;
pub const PSK_HANDLE_LEN: usize = 16;

pub struct PSQInitiator<T: PSQ> {
    signing_key: [u8; 32],
    verification_key: VerificationKey,
    state: Option<Initiator>,
    _t: PhantomData<T>,
}

impl<'a, T: PSQ> PSQInitiator<T> {
    pub fn init(signing_key: impl AsRef<[u8]>, verification_key: impl AsRef<[u8]>) -> Self {
        let mut sig_key: [u8; 32] = [0u8; 32];
        sig_key.clone_from_slice(signing_key.as_ref());

        let mut verif_key: [u8; 32] = [0u8; 32];
        verif_key.clone_from_slice(verification_key.as_ref());

        Self {
            signing_key: sig_key,
            verification_key: VerificationKey::from_bytes(verif_key),
            state: None,
            _t: PhantomData,
        }
    }

    pub fn compute_initiator_message<R>(
        &mut self,
        rng: &mut R,
        responder_kem_public_key: &<T::InnerKEM as KEM>::EncapsulationKey,
        context: &[u8; CONTEXT_LEN],
        psk_ttl: Duration,
    ) -> Result<Vec<u8>, PSQError>
    where
        <T as PSQ>::InnerKEM: KEM,
        R: RngCore + CryptoRng,
    {
        let (state, message) = Initiator::send_initial_message::<Ed25519, T>(
            context,
            psk_ttl,
            responder_kem_public_key,
            &self.signing_key,
            &self.verification_key,
            rng,
        )?;
        self.state = Some(state);

        let message_bytes = message.tls_serialize_detached()?;

        Ok(message_bytes)
    }

    // The initator generates the PSK themselves while computing the first message.
    // It's possible to get_psk() this after sending the first message.
    // The key will only be valid if we're sure that the responder has received it.
    //
    // If we use noise right after PSQ, we can just extract the PSK and expect the responder
    // to just send the first Noise message instead of responding to with a PSQ message.
    // In other words, instead of using the responder's PSQ message for key confirmation,
    // we could use a Noise handshake for key confirmation.

    // We need to ask Cryspen to make Initiator.k_pq to be accessible for this.

    pub fn get_psk(&self) -> Option<[u8; 32]> {
        match &self.state {
            Some(initiator_state) => Some(initiator_state.derive_unregistered_psk().unwrap()),
            None => None,
        }
    }

    pub fn finalize(&self, responder_message: &[u8]) -> Result<[u8; PSK_LENGTH], PSQError> {
        match &self.state {
            Some(state) => match ResponderMsg::tls_deserialize_exact(responder_message) {
                Ok(deserialized_responder_message) => {
                    match state.complete_handshake(&deserialized_responder_message) {
                        Ok(psk) => Ok(psk.psk),
                        Err(err) => Err(err.into()),
                    }
                }
                Err(err) => Err(err.into()),
            },

            None => Err(PSQError::IncorrectStateError),
        }
    }
}

pub struct PSQResponder<'a, T: PSQ>
where
    <T as PSQ>::InnerKEM: KEM,
{
    kem_private_key: &'a <T::InnerKEM as KEM>::DecapsulationKey,
    kem_public_key: &'a <T::InnerKEM as KEM>::EncapsulationKey,
    _t: PhantomData<T>,
}
impl<'a, T: PSQ> PSQResponder<'a, T>
where
    <T as PSQ>::InnerKEM: KEM,
{
    pub fn init(
        kem_private_key: &'a <T::InnerKEM as KEM>::DecapsulationKey,
        kem_public_key: &'a <T::InnerKEM as KEM>::EncapsulationKey,
    ) -> Self {
        Self {
            kem_private_key,
            kem_public_key,
            _t: PhantomData,
        }
    }

    pub fn compute_responder_message(
        &self,
        initiator_verification_key: &VerificationKey,
        initiator_message: &mut [u8],
        context: &[u8; CONTEXT_LEN],
        psk_ttl: Duration,
        psk_handle: &[u8; PSK_HANDLE_LEN],
    ) -> Result<([u8; PSK_LENGTH], Vec<u8>), PSQError> {
        let deserialized_initiator_message =
            InitiatorMsg::tls_deserialize_exact(initiator_message)?;

        let (registered_psk, responder_msg) = Responder::send::<Ed25519, T>(
            psk_handle,
            psk_ttl,
            context,
            self.kem_public_key,
            self.kem_private_key,
            &initiator_verification_key,
            &deserialized_initiator_message,
        )?;

        let responder_bytes = responder_msg.tls_serialize_detached()?;
        Ok((registered_psk.psk, responder_bytes))
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use libcrux_ed25519::VerificationKey;
    use libcrux_psq::impls::{MlKem768, XWingKemDraft06, X25519};
    use libcrux_psq::traits::PSQ;
    use libcrux_traits::kem::KEM;
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    use crate::psq::{CONTEXT_LEN, PSK_HANDLE_LEN};

    use super::{PSQInitiator, PSQResponder};

    fn test_helper<T: PSQ, R>(
        rng: &mut R,
        responder_kem_private_key: <T::InnerKEM as KEM>::DecapsulationKey,
        responder_kem_public_key: <T::InnerKEM as KEM>::EncapsulationKey,
    ) where
        R: CryptoRng + rand::RngCore,
    {
        // set PSQ TTL
        let psk_ttl: Duration = Duration::from_secs(3600);

        // generate random context string
        let mut context: [u8; CONTEXT_LEN] = [0u8; CONTEXT_LEN];
        rng.fill_bytes(&mut context);

        // generate random psk handle
        let mut psk_handle: [u8; PSK_HANDLE_LEN] = [0u8; PSK_HANDLE_LEN];
        rng.fill_bytes(&mut psk_handle);

        // generate ed25519 keys
        let mut secret_initiator: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut secret_initiator);
        let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

        let mut initiator: PSQInitiator<T> = PSQInitiator::init(
            initiator_ed25519_keypair.private_key().to_bytes(),
            initiator_ed25519_keypair.public_key().to_bytes(),
        );
        let responder: PSQResponder<T> =
            PSQResponder::init(&responder_kem_private_key, &responder_kem_public_key);

        let mut initiator_msg = initiator
            .compute_initiator_message(rng, &responder_kem_public_key, &context, psk_ttl)
            .unwrap();

        let pre_response_psk = initiator.get_psk().unwrap();

        let initiator_public_key =
            VerificationKey::from_bytes(initiator_ed25519_keypair.public_key().to_bytes());

        let (responder_psk, responder_msg) = responder
            .compute_responder_message(
                &initiator_public_key,
                &mut initiator_msg,
                &context,
                psk_ttl,
                &psk_handle,
            )
            .unwrap();

        let initiator_psk = initiator.finalize(&responder_msg).unwrap();

        assert_eq!(initiator_psk, pre_response_psk);
        assert_eq!(initiator_psk, responder_psk);
    }

    #[test]
    fn test_psq_e2e_mlkem() {
        let mut rng = rand::rng();

        // generate mlkem keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rng).unwrap();

        test_helper::<MlKem768, _>(
            &mut rng,
            responder_kem_private_key,
            responder_kem_public_key,
        );
    }

    #[test]
    fn test_psq_e2e_xwing() {
        let mut rng = rand::rng();

        // generate xwing keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::XWingKemDraft06, &mut rand::rng())
                .unwrap();

        test_helper::<XWingKemDraft06, _>(
            &mut rng,
            responder_kem_private_key,
            responder_kem_public_key,
        );
    }

    #[test]
    fn test_psq_e2e_dhkem() {
        let mut rng = rand::rng();

        // generate dhkem keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::X25519, &mut rand::rng()).unwrap();

        test_helper::<X25519, _>(
            &mut rng,
            responder_kem_private_key,
            responder_kem_public_key,
        );
    }
}
