use nym_contracts_common::{signing::MessageSignature, IdentityKey};
use nym_crypto::asymmetric::identity;
use nym_name_service_common::{
    signing_types::SignableNameRegisterMsg, Address, NameDetails, NymName,
};

use crate::test_helpers::signing::ed25519_sign_message;

pub struct TestName {
    pub name: NameDetails,
    pub id_keys: identity::KeyPair,
}

impl TestName {
    pub fn new(name: NymName, address: Address, id_keys: identity::KeyPair) -> Self {
        let identity_key = id_keys.public_key().to_base58_string();
        assert_eq!(
            identity_key,
            address.client_id().to_string(),
            "address and identity key must match"
        );
        let name = NameDetails {
            name,
            address,
            identity_key,
        };
        Self { name, id_keys }
    }

    pub fn identity_key(&self) -> &IdentityKey {
        &self.name.identity_key
    }

    pub fn details(&self) -> &NameDetails {
        &self.name
    }

    pub fn sign(self, payload: SignableNameRegisterMsg) -> SignedTestName {
        let owner_signature = ed25519_sign_message(payload, self.id_keys.private_key());
        SignedTestName {
            name: self.name,
            keys: self.id_keys,
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

    pub fn address(&self) -> &Address {
        &self.name.address
    }
}

impl From<SignedTestName> for NameDetails {
    fn from(signed_name: SignedTestName) -> Self {
        signed_name.name
    }
}
