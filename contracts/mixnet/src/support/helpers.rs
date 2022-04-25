// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnodes::storage as mixnodes_storage;
use crate::{constants, gateways::storage as gateways_storage};

use crate::error::ContractError;
use cosmwasm_std::{Addr, Deps, Storage};
use mixnet_contract_common::{reward_params::EpochRewardParams, IdentityKeyRef};

pub(crate) fn is_authorized(sender: String, storage: &dyn Storage) -> Result<(), ContractError> {
    if sender != crate::mixnet_contract_settings::storage::rewarding_validator_address(storage)? {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn epoch_reward_params(
    epoch_id: u32,
    storage: &mut dyn Storage,
) -> Result<EpochRewardParams, ContractError> {
    let state = crate::mixnet_contract_settings::storage::CONTRACT_STATE
        .load(storage)
        .map(|settings| settings.params)?;
    let reward_pool = crate::rewards::storage::REWARD_POOL.load(storage)?;
    let interval_reward_percent = crate::constants::INTERVAL_REWARD_PERCENT;
    let epochs_in_interval = crate::constants::EPOCHS_IN_INTERVAL;

    let epoch_reward_params = EpochRewardParams::new(
        (reward_pool.u128() / 100 / epochs_in_interval as u128) * interval_reward_percent as u128,
        state.mixnode_rewarded_set_size as u128,
        state.mixnode_active_set_size as u128,
        crate::rewards::storage::circulating_supply(storage)?.u128(),
        constants::SYBIL_RESISTANCE_PERCENT,
        constants::ACTIVE_SET_WORK_FACTOR,
    );

    crate::rewards::storage::EPOCH_REWARD_PARAMS.save(storage, epoch_id, &epoch_reward_params)?;

    Ok(epoch_reward_params)
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
    deps: Deps<'_>,
    owner: &Addr,
    signature: String,
    identity: IdentityKeyRef<'_>,
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
