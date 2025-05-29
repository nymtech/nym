// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, time::Duration};

use libcrux_psq::{
    cred::Ed25519,
    impls::MlKem768,
    psk_registration::{Initiator, InitiatorMsg, Responder, ResponderMsg},
    traits::PSQ,
};

use libcrux_traits::kem::KEM;
use nym_crypto::asymmetric::ed25519;

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

impl<'a> PSQInitiator<'a, MlKem768> {
    pub fn initiator_message(
        &mut self,
        responder_kem_public_key: &libcrux_kem::PublicKey,
    ) -> Result<InitiatorMsg<MlKem768>, PSQError> {
        let (state, message) = Initiator::send_initial_message::<Ed25519, MlKem768>(
            &[0u8],
            Duration::from_secs(3600),
            &responder_kem_public_key,
            &self.signing_keypair.private_key().to_bytes(),
            &self.signing_keypair.public_key().to_bytes(),
            &mut rand::rng(),
        )?;
        self.state = Some(state);
        Ok(message)
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
}
impl<'a> PSQResponder<'a, MlKem768> {
    pub fn responder_msg(
        &self,
        initiator_verification_key: &'a ed25519::PublicKey,
        initiator_message: &InitiatorMsg<MlKem768>,
    ) -> Result<([u8; PSK_LENGTH], ResponderMsg), PSQError> {
        let (registered_psk, responder_msg) = Responder::send::<Ed25519, MlKem768>(
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
    use libcrux_psq::impls::MlKem768;
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    use super::{PSQInitiator, PSQResponder};

    #[test]
    fn test_psq_e2e() {
        // generate ed25519 keys
        let mut secret_initiator: [u8; 32] = [0u8; 32];
        rand::rng().fill_bytes(&mut secret_initiator);
        let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

        // generate kem keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rand::rng()).unwrap();

        let mut initiator: PSQInitiator<MlKem768> = PSQInitiator::init(&initiator_ed25519_keypair);
        let responder: PSQResponder<MlKem768> =
            PSQResponder::init(&responder_kem_private_key, &responder_kem_public_key);

        let initiator_msg = initiator
            .initiator_message(&responder_kem_public_key)
            .unwrap();

        let (responder_psk, responder_msg) = responder
            .responder_msg(initiator_ed25519_keypair.public_key(), &initiator_msg)
            .unwrap();

        let initiator_psk = initiator.finalize(&responder_msg).unwrap();

        assert_eq!(initiator_psk, responder_psk);
    }
}
