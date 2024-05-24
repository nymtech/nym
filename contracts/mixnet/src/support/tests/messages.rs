use cosmwasm_std::{Coin, Deps};
use mixnet_contract_common::{ExecuteMsg, Gateway, IdentityKey};
use nym_crypto::asymmetric::identity;
use rand_chacha::rand_core::{CryptoRng, RngCore};

use crate::support::tests;
use crate::support::tests::test_helpers::{ed25519_sign_message, gateway_bonding_sign_payload};

pub(crate) fn valid_bond_gateway_msg(
    mut rng: impl RngCore + CryptoRng,
    deps: Deps<'_>,
    stake: Vec<Coin>,
    sender: &str,
) -> (ExecuteMsg, IdentityKey) {
    let keypair = identity::KeyPair::new(&mut rng);
    let identity_key = keypair.public_key().to_base58_string();
    let legit_sphinx_keys = nym_crypto::asymmetric::encryption::KeyPair::new(&mut rng);

    let gateway = Gateway {
        identity_key,
        sphinx_key: legit_sphinx_keys.public_key().to_base58_string(),
        ..tests::fixtures::gateway_fixture()
    };

    let msg = gateway_bonding_sign_payload(deps, sender, gateway.clone(), stake);
    let owner_signature = ed25519_sign_message(msg, keypair.private_key());

    let identity_key = keypair.public_key().to_base58_string();
    (
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        },
        identity_key,
    )
}
