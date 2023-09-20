use cosmwasm_std::{Addr, Coin, Deps};
use nym_contracts_common::signing::{
    MessageSignature, SignableMessage, SigningAlgorithm, SigningPurpose,
};
use nym_crypto::asymmetric::identity;
use nym_name_service_common::{
    signing_types::{construct_name_register_sign_payload, SignableNameRegisterMsg},
    NameDetails,
};
use serde::Serialize;

use crate::state;

pub fn name_register_sign_payload(
    deps: Deps<'_>,
    owner: &str,
    name: NameDetails,
    deposit: Coin,
) -> SignableNameRegisterMsg {
    let owner = Addr::unchecked(owner);
    let nonce = state::get_signing_nonce(deps.storage, owner.clone()).unwrap();
    construct_name_register_sign_payload(nonce, owner, deposit, name)
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
