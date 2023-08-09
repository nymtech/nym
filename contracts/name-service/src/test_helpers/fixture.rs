use cosmwasm_std::{Addr, Coin, DepsMut};
use nym_contracts_common::{signing::MessageSignature, IdentityKeyRef};
use nym_crypto::asymmetric::identity;
use nym_name_service_common::{Address, NameDetails, NameId, NymName, RegisteredName};
use rand_chacha::rand_core::{CryptoRng, RngCore};

use super::{
    helpers::nyms,
    signing::{ed25519_sign_message, name_register_sign_payload},
};

pub fn new_name(
    name_id: NameId,
    name: &NymName,
    address: &Address,
    owner: &Addr,
    identity_key: IdentityKeyRef,
) -> RegisteredName {
    RegisteredName {
        id: name_id,
        name: NameDetails {
            name: name.clone(),
            address: address.clone(),
            identity_key: identity_key.to_string(),
        },
        owner: owner.clone(),
        block_height: 12345,
        deposit: nyms(100),
    }
}

pub fn name_fixture(id: NameId) -> RegisteredName {
    new_name(
        id,
        &NymName::new("my-service").unwrap(),
        &Address::new("client_id.client_key@gateway_id"),
        &Addr::unchecked("steve"),
        "identity",
    )
}

#[allow(unused)]
pub fn name_fixture_with_name(id: NameId, name: &str, address: &str) -> RegisteredName {
    new_name(
        id,
        &NymName::new(name).unwrap(),
        &Address::new(address),
        &Addr::unchecked("steve"),
        "identity",
    )
}

pub fn name_fixture_full(id: NameId, name: &str, address: &str, owner: &str) -> RegisteredName {
    new_name(
        id,
        &NymName::new(name).unwrap(),
        &Address::new(address),
        &Addr::unchecked(owner),
        "identity",
    )
}

// Create a new name, using a correctly generted identity key
pub fn new_name_details<R>(
    rng: &mut R,
    name: &str,
    nym_address: &str,
) -> (NameDetails, identity::KeyPair)
where
    R: RngCore + CryptoRng,
{
    let keypair = identity::KeyPair::new(rng);
    (
        NameDetails {
            name: NymName::new(name).unwrap(),
            address: Address::new(nym_address),
            identity_key: keypair.public_key().to_base58_string(),
        },
        keypair,
    )
}

// Create a new service, with a correctly generated identity key, and sign it
pub fn new_name_details_with_sign<R>(
    deps: DepsMut<'_>,
    rng: &mut R,
    name: &str,
    nym_address: &str,
    owner: &str,
    deposit: Coin,
) -> (NameDetails, MessageSignature)
where
    R: RngCore + CryptoRng,
{
    // Service
    let (name, keypair) = new_name_details(rng, name, nym_address);

    // Sign
    let sign_msg = name_register_sign_payload(deps.as_ref(), owner, name.clone(), deposit);
    let owner_signature = ed25519_sign_message(sign_msg, keypair.private_key());

    (name, owner_signature)
}
