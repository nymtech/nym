use cosmwasm_std::{Addr, Coin, DepsMut};
use nym_contracts_common::{signing::MessageSignature, IdentityKeyRef};
use nym_crypto::asymmetric::{encryption, identity};
use nym_name_service_common::{Address, NameDetails, NameId, NymName, RegisteredName};
use nym_sphinx_addressing::clients::Recipient;
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
        &Address::new("client_id.client_key@gateway_id").unwrap(),
        &Addr::unchecked("steve"),
        "identity",
    )
}

pub fn name_fixture_full(id: NameId, name: &str, address: &str, owner: &str) -> RegisteredName {
    new_name(
        id,
        &NymName::new(name).unwrap(),
        &Address::new(address).unwrap(),
        &Addr::unchecked(owner),
        "identity",
    )
}

fn new_recipient<R>(rng: &mut R) -> (Recipient, identity::KeyPair)
where
    R: RngCore + CryptoRng,
{
    let client_id_keys = identity::KeyPair::new(rng);
    let client_enc_keys = encryption::KeyPair::new(rng);
    let gateway_id_keys = identity::KeyPair::new(rng);

    (
        Recipient::new(
            *client_id_keys.public_key(),
            *client_enc_keys.public_key(),
            *gateway_id_keys.public_key(),
        ),
        client_id_keys,
    )
}

pub fn new_address<R>(rng: &mut R) -> (Address, identity::KeyPair)
where
    R: RngCore + CryptoRng,
{
    let (recipient, client_id_keys) = new_recipient(rng);
    let address = Address::new(&recipient.to_string()).unwrap();
    (address, client_id_keys)
}

// Create a new name with a correctly generated nym address and matching identity key
pub fn new_name_details<R>(rng: &mut R, name: &str) -> (NameDetails, identity::KeyPair)
where
    R: RngCore + CryptoRng,
{
    let (address, client_id_keys) = new_address(rng);
    let identity_key = client_id_keys.public_key().to_base58_string();

    (
        NameDetails {
            name: NymName::new(name).unwrap(),
            address,
            identity_key,
        },
        client_id_keys,
    )
}

// Create a new name, with a correctly generated nym adress and identity key, and sign it
pub fn new_name_details_with_sign<R>(
    deps: DepsMut<'_>,
    rng: &mut R,
    name: &str,
    owner: &str,
    deposit: Coin,
) -> (NameDetails, MessageSignature)
where
    R: RngCore + CryptoRng,
{
    // Name
    let (name, client_id_keys) = new_name_details(rng, name);

    // Sign
    let sign_msg = name_register_sign_payload(deps.as_ref(), owner, name.clone(), deposit);
    let owner_signature = ed25519_sign_message(sign_msg, client_id_keys.private_key());

    (name, owner_signature)
}

#[cfg(test)]
mod test {
    use crate::test_helpers::helpers;

    use super::*;

    #[test]
    fn new_recipient_creates_matching_identity_and_client_id_public_key() {
        let mut test_rng = helpers::test_rng();
        let (recipient, client_id_keys) = new_recipient(&mut test_rng);
        assert_eq!(recipient.identity(), client_id_keys.public_key());

        // A test not for our code, but that we actually confirm our understanding of Recipient.
        // One might view it as unnecessary, but it's a good sanity check for such a core assumption.
        let recipient_str = format!(
            "{}.{}@{}",
            recipient.identity().to_base58_string(),
            recipient.encryption_key().to_base58_string(),
            recipient.gateway().to_base58_string()
        );
        assert_eq!(recipient.to_string(), recipient_str);
    }
}
