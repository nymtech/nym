// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, time::Duration};

use libcrux_psq::{
    cred::Ed25519,
    psk_registration::{Initiator, InitiatorMsg, Responder, ResponderMsg},
    traits::{Decode, Encode, PSQ},
};

use libcrux_traits::kem::KEM;
use nym_crypto::asymmetric::ed25519;
use rand::CryptoRng;

use crate::error::PSQError;

pub const PSK_LENGTH: usize = 32;
pub const CONTEXT_LEN: usize = 16;
pub const PSK_HANDLE_LEN: usize = 16;

pub struct PSQInitiator<'a, T: PSQ> {
    signing_keypair: &'a ed25519::KeyPair,
    state: Option<Initiator>,
    _t: PhantomData<T>,
}

impl<'a, T: PSQ> PSQInitiator<'a, T> {
    pub fn init(signing_keypair: &'a ed25519::KeyPair) -> Self {
        Self {
            signing_keypair,
            state: None,
            _t: PhantomData,
        }
    }

    pub fn compute_initiator_message(
        &mut self,
        rng: &mut impl CryptoRng,
        responder_kem_public_key: &'a <T::InnerKEM as KEM>::EncapsulationKey,
        context: &[u8; CONTEXT_LEN],
        psk_ttl: Duration,
    ) -> Result<InitiatorMsg<T::InnerKEM>, PSQError> {
        let (state, message) = Initiator::send_initial_message::<Ed25519, T>(
            context,
            psk_ttl,
            &responder_kem_public_key,
            &self.signing_keypair.private_key().to_bytes(),
            &self.signing_keypair.public_key().to_bytes(),
            rng,
        )?;
        self.state = Some(state);

        let initiator_msg_bytes: Vec<u8> = message.encode();

        // This only works for MlKem768
        // let decoded_msg: InitiatorMsg<T::InnerKEM> = InitiatorMsg::decode(&initiator_msg_bytes)?.0;

        Ok(message)
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

    // pub fn get_psk(&self) -> Option<[u8; 32]> {
    //     match self.state {
    //         Some(initiator_state) => initiator_state.k_pq,
    //         None => None,
    //     }
    // }

    pub fn finalize(&self, responder_message: &ResponderMsg) -> Result<[u8; PSK_LENGTH], PSQError> {
        match &self.state {
            Some(state) => match state.complete_handshake(responder_message) {
                Ok(psk) => Ok(psk.psk),
                Err(err) => Err(err.into()),
            },
            None => Err(PSQError::IncorrectStateError),
        }
    }
}

pub struct PSQResponder<'a, T: PSQ> {
    kem_private_key: &'a <T::InnerKEM as KEM>::DecapsulationKey,
    kem_public_key: &'a <T::InnerKEM as KEM>::EncapsulationKey,
    _t: PhantomData<T>,
}
impl<'a, T: PSQ> PSQResponder<'a, T> {
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
        initiator_verification_key: &'a ed25519::PublicKey,
        initiator_message: &InitiatorMsg<T::InnerKEM>,
        context: &[u8; CONTEXT_LEN],
        psk_ttl: Duration,
        psk_handle: &[u8; PSK_HANDLE_LEN],
    ) -> Result<([u8; PSK_LENGTH], ResponderMsg), PSQError> {
        let (registered_psk, responder_msg) = Responder::send::<Ed25519, T>(
            psk_handle,
            psk_ttl,
            context,
            self.kem_public_key,
            self.kem_private_key,
            &initiator_verification_key.to_bytes(),
            initiator_message,
        )?;

        Ok((registered_psk.psk, responder_msg))
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use libcrux_psq::impls::{MlKem768, XWingKemDraft06, X25519};
    use libcrux_psq::psk_registration::{Initiator, InitiatorMsg, Responder};
    use libcrux_psq::traits::{Decode, Encode, PSQ};
    use libcrux_traits::kem::KEM;
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    use crate::psq::{CONTEXT_LEN, PSK_HANDLE_LEN};

    use super::{PSQInitiator, PSQResponder};

    fn test_helper<T: PSQ>(
        rng: &mut impl CryptoRng,
        responder_kem_private_key: <T::InnerKEM as KEM>::DecapsulationKey,
        responder_kem_public_key: <T::InnerKEM as KEM>::EncapsulationKey,
    ) {
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

        let mut initiator: PSQInitiator<T> = PSQInitiator::init(&initiator_ed25519_keypair);
        let responder: PSQResponder<T> =
            PSQResponder::init(&responder_kem_private_key, &responder_kem_public_key);

        let initiator_msg = initiator
            .compute_initiator_message(rng, &responder_kem_public_key, &context, psk_ttl)
            .unwrap();

        let initiator_msg_bytes = initiator_msg.encode();

        // let decoded_initiator_msg: (InitiatorMsg<T::InnerKEM>, usize) =
        //     InitiatorMsg::decode(&initiator_msg_bytes).unwrap();

        let (responder_psk, responder_msg) = responder
            .compute_responder_message(
                initiator_ed25519_keypair.public_key(),
                &initiator_msg,
                &context,
                psk_ttl,
                &psk_handle,
            )
            .unwrap();

        let initiator_psk = initiator.finalize(&responder_msg).unwrap();

        assert_eq!(initiator_psk, responder_psk);
    }

    #[test]
    fn test_psq_e2e_mlkem() {
        let mut rng = rand::rng();

        // generate mlkem keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rng).unwrap();

        test_helper::<MlKem768>(
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

        test_helper::<XWingKemDraft06>(
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

        test_helper::<X25519>(
            &mut rng,
            responder_kem_private_key,
            responder_kem_public_key,
        );
    }
}
