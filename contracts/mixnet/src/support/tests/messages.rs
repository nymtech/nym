use mixnet_contract_common::{ExecuteMsg, Gateway, IdentityKey};
use rand_chacha::rand_core::{CryptoRng, RngCore};

use crate::support::tests;

pub(crate) fn valid_bond_gateway_msg(
    mut rng: impl RngCore + CryptoRng,
    sender: &str,
) -> (ExecuteMsg, IdentityKey) {
    let keypair = crypto::asymmetric::identity::KeyPair::new(&mut rng);
    let owner_signature = keypair
        .private_key()
        .sign(sender.as_bytes())
        .to_base58_string();

    let identity_key = keypair.public_key().to_base58_string();
    (
        ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: identity_key.clone(),
                ..tests::fixtures::gateway_fixture()
            },
            owner_signature,
        },
        identity_key,
    )
}
