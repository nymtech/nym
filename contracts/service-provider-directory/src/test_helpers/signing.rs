use cosmwasm_std::{Addr, Coin, Deps};
use nym_contracts_common::signing::{
    MessageSignature, SignableMessage, SigningAlgorithm, SigningPurpose,
};
use nym_crypto::asymmetric::identity;
use nym_service_provider_directory_common::{
    signing_types::{
        construct_service_provider_announce_sign_payload, SignableServiceProviderAnnounceMsg,
    },
    ServiceDetails,
};
use serde::Serialize;

use crate::state;

pub fn service_provider_announce_sign_payload(
    deps: Deps<'_>,
    owner: &str,
    service: ServiceDetails,
    deposit: Coin,
) -> SignableServiceProviderAnnounceMsg {
    let owner = Addr::unchecked(owner);
    let nonce = state::get_signing_nonce(deps.storage, owner.clone()).unwrap();
    construct_service_provider_announce_sign_payload(nonce, owner, deposit, service)
}

pub fn ed25519_sign_message<T: Serialize + SigningPurpose>(
    message: SignableMessage<T>,
    private_key: &identity::PrivateKey,
) -> MessageSignature {
    match message.algorithm {
        SigningAlgorithm::Ed25519 => {
            let plaintext = message.to_plaintext().unwrap();
            let signature = private_key.sign(plaintext);
            MessageSignature::from(signature.to_bytes().as_ref())
        }
        SigningAlgorithm::Secp256k1 => {
            unimplemented!()
        }
    }
}
