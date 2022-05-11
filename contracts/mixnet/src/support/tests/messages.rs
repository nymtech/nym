use mixnet_contract_common::{ExecuteMsg, Gateway, IdentityKey, MixNode, SphinxKey};
use rand::thread_rng;

use crate::support::tests;

pub(crate) fn valid_bond_mixnode_msg(sender: &str) -> (ExecuteMsg, (IdentityKey, SphinxKey)) {
    let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
    let legit_sphinx_key = crypto::asymmetric::encryption::KeyPair::new(&mut thread_rng());
    let owner_signature = keypair
        .private_key()
        .sign(sender.as_bytes())
        .to_base58_string();

    let identity_key = keypair.public_key().to_base58_string();
    let sphinx_key = legit_sphinx_key.public_key().to_base58_string();
    (
        ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: identity_key.clone(),
                sphinx_key: sphinx_key.clone(),
                ..tests::fixtures::mix_node_fixture()
            },
            owner_signature,
        },
        (identity_key, sphinx_key),
    )
}

pub(crate) fn valid_bond_gateway_msg(sender: &str) -> (ExecuteMsg, IdentityKey) {
    let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
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
