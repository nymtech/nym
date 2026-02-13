// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_ml_kem::mlkem768::{MlKem768KeyPair, MlKem768PrivateKey, MlKem768PublicKey};
use libcrux_psq::classic_mceliece;
use nym_kkt_ciphersuite::KEM;
use std::fmt::{Debug, Formatter};

/// Wrapper around keys used for the KEM exchange
pub struct KEMKeys {
    mc_eliece: classic_mceliece::KeyPair,
    ml_kem768: MlKem768KeyPair,
}

impl Debug for KEMKeys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KEMKeys")
            .field("mc_eliece", &"<redacted>")
            .field("ml_kem768", &"<redacted>")
            .finish()
    }
}

impl KEMKeys {
    pub fn new(mc_eliece: classic_mceliece::KeyPair, ml_kem768: MlKem768KeyPair) -> Self {
        Self {
            mc_eliece,
            ml_kem768,
        }
    }

    pub fn encoded_encapsulation_key(&self, kem: KEM) -> Option<&[u8]> {
        match kem {
            KEM::McEliece => Some(self.mc_eliece.pk.as_ref()),
            KEM::MlKem768 => Some(self.ml_kem768.pk()),
            _ => None,
        }
    }

    pub fn mc_eliece_encapsulation_key(&self) -> &classic_mceliece::PublicKey {
        &self.mc_eliece.pk
    }

    pub fn ml_kem768_encapsulation_key(&self) -> &MlKem768PublicKey {
        self.ml_kem768.public_key()
    }

    pub fn mc_eliece_decapsulation_key(&self) -> &classic_mceliece::SecretKey {
        &self.mc_eliece.sk
    }

    pub fn ml_kem768_decapsulation_key(&self) -> &MlKem768PrivateKey {
        self.ml_kem768.private_key()
    }
}
