use nym_contracts_common::{signing::MessageSignature, IdentityKey};
use nym_crypto::asymmetric::identity;
use nym_name_service_common::{
    signing_types::SignableNameRegisterMsg, Address, NameDetails, NymName,
};
use rand_chacha::ChaCha20Rng;

use crate::test_helpers::signing::ed25519_sign_message;

pub struct TestName {
    pub name: NameDetails,
    pub keys: identity::KeyPair,
    pub rng: ChaCha20Rng,
}

impl TestName {
    pub fn new(rng: &mut ChaCha20Rng, name: NymName, address: Address) -> Self {
        let keys = identity::KeyPair::new(rng);
        let name = NameDetails {
            name,
            address,
            identity_key: keys.public_key().to_base58_string(),
        };
        Self {
            name,
            keys,
            rng: rng.clone(),
        }
    }

    pub fn identity_key(&self) -> &IdentityKey {
        &self.name.identity_key
    }

    pub fn details(&self) -> &NameDetails {
        &self.name
    }

    pub fn sign(self, payload: SignableNameRegisterMsg) -> SignedTestName {
        let owner_signature = ed25519_sign_message(payload, self.keys.private_key());
        SignedTestName {
            name: self.name,
            keys: self.keys,
            owner_signature,
        }
    }
}

impl From<TestName> for NameDetails {
    fn from(test_name: TestName) -> Self {
        test_name.name
    }
}

pub struct SignedTestName {
    pub name: NameDetails,
    pub keys: identity::KeyPair,
    pub owner_signature: MessageSignature,
}

impl SignedTestName {
    pub fn identity_key(&self) -> &IdentityKey {
        &self.name.identity_key
    }

    pub fn details(&self) -> &NameDetails {
        &self.name
    }
}

impl From<SignedTestName> for NameDetails {
    fn from(signed_name: SignedTestName) -> Self {
        signed_name.name
    }
}
