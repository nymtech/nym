use nym_contracts_common::{signing::MessageSignature, IdentityKey};
use nym_crypto::asymmetric::identity;
use nym_service_provider_directory_common::{
    signing_types::SignableServiceProviderAnnounceMsg, NymAddress, ServiceDetails, ServiceType,
};
use rand_chacha::ChaCha20Rng;

use crate::test_helpers::signing::ed25519_sign_message;

pub struct TestService {
    pub service: ServiceDetails,
    pub keys: identity::KeyPair,
}

impl TestService {
    pub fn new(rng: &mut ChaCha20Rng, nym_address: NymAddress) -> Self {
        let keys = identity::KeyPair::new(rng);
        let service = ServiceDetails {
            nym_address,
            service_type: ServiceType::NetworkRequester,
            identity_key: keys.public_key().to_base58_string(),
        };
        Self { service, keys }
    }

    pub fn identity_key(&self) -> &IdentityKey {
        &self.service.identity_key
    }

    pub fn details(&self) -> &ServiceDetails {
        &self.service
    }

    pub fn sign(self, payload: SignableServiceProviderAnnounceMsg) -> SignedTestService {
        let owner_signature = ed25519_sign_message(payload, self.keys.private_key());
        SignedTestService {
            service: self.service,
            keys: self.keys,
            owner_signature,
        }
    }
}

impl From<TestService> for ServiceDetails {
    fn from(test_service: TestService) -> Self {
        test_service.service
    }
}

pub struct SignedTestService {
    pub service: ServiceDetails,
    pub keys: identity::KeyPair,
    pub owner_signature: MessageSignature,
}

impl SignedTestService {
    pub fn identity_key(&self) -> &IdentityKey {
        &self.service.identity_key
    }

    pub fn details(&self) -> &ServiceDetails {
        &self.service
    }
}

impl From<SignedTestService> for ServiceDetails {
    fn from(signed_service: SignedTestService) -> Self {
        signed_service.service
    }
}
