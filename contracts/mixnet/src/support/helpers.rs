// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::gateways::storage as gateways_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{Addr, Deps, Storage};
use mixnet_contract::IdentityKeyRef;

pub fn generate_storage_key(address: &Addr, proxy: Option<&Addr>) -> Vec<u8> {
    if let Some(proxy) = &proxy {
        address
            .as_bytes()
            .iter()
            .zip(proxy.as_bytes())
            .map(|(x, y)| x ^ y)
            .collect()
    } else {
        address.as_bytes().to_vec()
    }
}

// check if the target address has already bonded a mixnode or gateway,
// in either case, return an appropriate error
pub(crate) fn ensure_no_existing_bond(
    storage: &dyn Storage,
    sender: &Addr,
) -> Result<(), ContractError> {
    if mixnodes_storage::mixnodes()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsMixnode);
    }

    if gateways_storage::gateways()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsGateway);
    }

    Ok(())
}

pub(crate) fn validate_node_identity_signature(
    deps: Deps,
    owner: &Addr,
    signature: String,
    identity: IdentityKeyRef,
) -> Result<(), ContractError> {
    let owner_bytes = owner.as_bytes();

    let mut identity_bytes = [0u8; 32];
    let mut signature_bytes = [0u8; 64];

    let identity_used_bytes = bs58::decode(identity)
        .into(&mut identity_bytes)
        .map_err(|err| ContractError::MalformedEd25519IdentityKey(err.to_string()))?;
    let signature_used_bytes = bs58::decode(signature)
        .into(&mut signature_bytes)
        .map_err(|err| ContractError::MalformedEd25519Signature(err.to_string()))?;

    if identity_used_bytes != 32 {
        return Err(ContractError::MalformedEd25519IdentityKey(
            "Too few bytes provided".into(),
        ));
    }

    if signature_used_bytes != 64 {
        return Err(ContractError::MalformedEd25519Signature(
            "Too few bytes provided".into(),
        ));
    }

    let res = deps
        .api
        .ed25519_verify(owner_bytes, &signature_bytes, &identity_bytes)
        .map_err(cosmwasm_std::StdError::verification_err)?;
    if !res {
        Err(ContractError::InvalidEd25519Signature)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use crypto::asymmetric::identity;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn validating_node_signature() {
        let deps = mock_dependencies();

        // since those tests are NOT compiled to wasm, we can use rng-related dependency
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let short_bs58 = "2SfEgZ4aQUr3HSwqE";
        let long_bs58 = "g34PyULki9fc3FqKobj5wdVNCaWAt1M9oZowyyMFfWSCejxg7wt574piZVjqjFEN2UXsgZ56KTkKf3jnWD4DJ2Gsf7KXQAvptFfcYRrZHTjMVo3NXcBSNm3wDBKZWZURzp4Fixv";

        let address1 = Addr::unchecked("some-dummy-address1");
        let address2 = Addr::unchecked("some-dummy-address2");

        let keypair1 = identity::KeyPair::new(&mut rng);
        let keypair2 = identity::KeyPair::new(&mut rng);

        let sig_addr1_key1 = keypair1
            .private_key()
            .sign(address1.as_bytes())
            .to_base58_string();
        let sig_addr2_key1 = keypair1
            .private_key()
            .sign(address2.as_bytes())
            .to_base58_string();
        let sig_addr1_key2 = keypair2
            .private_key()
            .sign(address1.as_bytes())
            .to_base58_string();

        assert_eq!(
            Err(ContractError::MalformedEd25519IdentityKey(
                "buffer provided to decode base58 encoded string into was too small".into()
            )),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr1_key1.clone(),
                long_bs58,
            )
        );

        assert_eq!(
            Err(ContractError::MalformedEd25519Signature(
                "buffer provided to decode base58 encoded string into was too small".into()
            )),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                long_bs58.into(),
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(ContractError::MalformedEd25519IdentityKey(
                "Too few bytes provided".into()
            )),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr1_key1.clone(),
                short_bs58,
            )
        );

        assert_eq!(
            Err(ContractError::MalformedEd25519Signature(
                "Too few bytes provided".into()
            )),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                short_bs58.into(),
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(ContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr1_key1.clone(),
                &keypair2.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(ContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr2_key1,
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(ContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address2,
                sig_addr1_key1.clone(),
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(ContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr1_key2,
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert!(validate_node_identity_signature(
            deps.as_ref(),
            &address1,
            sig_addr1_key1,
            &keypair1.public_key().to_base58_string(),
        )
        .is_ok());
    }
}
