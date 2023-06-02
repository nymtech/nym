use cosmwasm_std::{Addr, Coin, DepsMut};
use nym_contracts_common::{signing::MessageSignature, IdentityKeyRef};
use nym_crypto::asymmetric::identity;
use nym_service_provider_directory_common::{
    NymAddress, Service, ServiceDetails, ServiceId, ServiceType,
};
use rand_chacha::rand_core::{CryptoRng, RngCore};

use super::{
    helpers::nyms,
    signing::{ed25519_sign_message, service_provider_announce_sign_payload},
};

pub fn new_service(
    service_id: ServiceId,
    nym_address: &NymAddress,
    announcer: &Addr,
    identity_key: IdentityKeyRef,
) -> Service {
    Service {
        service_id,
        service: ServiceDetails {
            nym_address: nym_address.clone(),
            service_type: ServiceType::NetworkRequester,
            identity_key: identity_key.to_string(),
        },
        announcer: announcer.clone(),
        block_height: 12345,
        deposit: nyms(100),
    }
}

pub fn service_fixture(service_id: ServiceId) -> Service {
    new_service(
        service_id,
        &NymAddress::new("nym"),
        &Addr::unchecked("steve"),
        "identity",
    )
}

pub fn service_fixture_with_address(service_id: ServiceId, nym_address: &str) -> Service {
    new_service(
        service_id,
        &NymAddress::new(nym_address),
        &Addr::unchecked("steve"),
        "identity",
    )
}

// Create a new service, using a correctly generted identity key
pub fn new_service_details<R>(rng: &mut R, nym_address: &str) -> (ServiceDetails, identity::KeyPair)
where
    R: RngCore + CryptoRng,
{
    let keypair = identity::KeyPair::new(rng);
    (
        ServiceDetails {
            nym_address: NymAddress::new(nym_address),
            service_type: ServiceType::NetworkRequester,
            identity_key: keypair.public_key().to_base58_string(),
        },
        keypair,
    )
}

// Create a new service, with a correctly generated identity key, and sign it
pub fn new_service_details_with_sign<R>(
    deps: DepsMut<'_>,
    rng: &mut R,
    nym_address: &str,
    announcer: &str,
    deposit: Coin,
) -> (ServiceDetails, MessageSignature)
where
    R: RngCore + CryptoRng,
{
    // Service
    let (service, keypair) = new_service_details(rng, nym_address);

    // Sign
    let sign_msg =
        service_provider_announce_sign_payload(deps.as_ref(), announcer, service.clone(), deposit);
    let owner_signature = ed25519_sign_message(sign_msg, keypair.private_key());

    (service, owner_signature)
}
