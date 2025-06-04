// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, time::Duration};

use libcrux_psq::{
    cred::Ed25519,
    psk_registration::{Initiator, InitiatorMsg, Responder, ResponderMsg},
    traits::PSQ,
};

use libcrux_traits::kem::KEM;
use nym_crypto::asymmetric::ed25519;
use rand::CryptoRng;

use crate::error::PSQError;

pub const PSK_LENGTH: usize = 32;

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

    pub fn initiator_message(
        &mut self,
        rng: &mut impl CryptoRng,
        responder_kem_public_key: &'a <T::InnerKEM as KEM>::EncapsulationKey,
    ) -> Result<InitiatorMsg<T::InnerKEM>, PSQError> {
        let (state, message) = Initiator::send_initial_message::<Ed25519, T>(
            &[0u8],
            Duration::from_secs(3600),
            &responder_kem_public_key,
            &self.signing_keypair.private_key().to_bytes(),
            &self.signing_keypair.public_key().to_bytes(),
            rng,
        )?;
        self.state = Some(state);
        Ok(message)
    }

    pub fn finalize(&self, responder_message: &ResponderMsg) -> Result<[u8; PSK_LENGTH], PSQError> {
        match &self.state {
            Some(state) => match state.complete_handshake(responder_message) {
                Ok(psk) => Ok(psk.psk),
                Err(err) => Err(PSQError::KEMError),
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

    pub fn responder_msg(
        &self,
        initiator_verification_key: &'a ed25519::PublicKey,
        initiator_message: &InitiatorMsg<T::InnerKEM>,
    ) -> Result<([u8; PSK_LENGTH], ResponderMsg), PSQError> {
        let (registered_psk, responder_msg) = Responder::send::<Ed25519, T>(
            &[0u8],
            Duration::from_secs(3600),
            &[0u8],
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
    use libcrux_psq::impls::{MlKem768, XWingKemDraft06, X25519};
    use libcrux_psq::traits::PSQ;
    use libcrux_traits::kem::KEM;
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    use super::{PSQInitiator, PSQResponder};

    fn test_helper<T: PSQ>(
        rng: &mut impl CryptoRng,
        responder_kem_private_key: <T::InnerKEM as KEM>::DecapsulationKey,
        responder_kem_public_key: <T::InnerKEM as KEM>::EncapsulationKey,
    ) {
        // generate ed25519 keys

        let mut secret_initiator: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut secret_initiator);
        let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

        let mut initiator: PSQInitiator<T> = PSQInitiator::init(&initiator_ed25519_keypair);
        let responder: PSQResponder<T> =
            PSQResponder::init(&responder_kem_private_key, &responder_kem_public_key);

        let initiator_msg = initiator
            .initiator_message(rng, &responder_kem_public_key)
            .unwrap();

        let (responder_psk, responder_msg) = responder
            .responder_msg(initiator_ed25519_keypair.public_key(), &initiator_msg)
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

        // generate mlkem keypair
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

        // generate mlkem keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::X25519, &mut rand::rng()).unwrap();

        test_helper::<X25519>(
            &mut rng,
            responder_kem_private_key,
            responder_kem_public_key,
        );
    }
}
