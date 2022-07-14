// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::gateways::storage as gateways_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{Addr, Api, BankMsg, Coin, CosmosMsg, Deps, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{IdentityKeyRef, MixNodeBond};

// helper trait to attach `Msg` to a response if it's provided
pub(crate) trait AttachOptionalMessage<T> {
    fn add_optional_message(self, msg: Option<impl Into<CosmosMsg<T>>>) -> Self;
}

impl<T> AttachOptionalMessage<T> for Response<T> {
    fn add_optional_message(self, msg: Option<impl Into<CosmosMsg<T>>>) -> Self {
        if let Some(msg) = msg {
            self.add_message(msg)
        } else {
            self
        }
    }
}

pub fn debug_with_visibility<S: Into<String>>(api: &dyn Api, msg: S) {
    api.debug(&*format!("\n\n\n=========================================\n{}\n=========================================\n\n\n", msg.into()));
}

/// Attempts to construct a `BankMsg` to send specified tokens to the provided
/// proxy address. If that's unavailable, the `BankMsg` will use the "owner" as the
/// "to_address".
pub(crate) fn send_to_proxy_or_owner(
    proxy: &Option<Addr>,
    owner: &Addr,
    amount: Vec<Coin>,
) -> BankMsg {
    BankMsg::Send {
        to_address: proxy.as_ref().unwrap_or(&owner).to_string(),
        amount,
    }
}

pub(crate) fn validate_pledge(
    mut pledge: Vec<Coin>,
    minimum_pledge: Coin,
) -> Result<Coin, MixnetContractError> {
    // check if anything was put as bond
    if pledge.is_empty() {
        return Err(MixnetContractError::NoBondFound);
    }

    if pledge.len() > 1 {
        return Err(MixnetContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if pledge[0].denom != minimum_pledge.denom {
        return Err(MixnetContractError::WrongDenom {
            received: pledge[0].denom.clone(),
            expected: minimum_pledge.denom,
        });
    }

    // check that the pledge contains the minimum amount of tokens
    if pledge[0].amount < minimum_pledge.amount {
        return Err(MixnetContractError::InsufficientPledge {
            received: pledge[0].clone(),
            minimum: minimum_pledge,
        });
    }

    Ok(pledge.pop().unwrap())
}

pub(crate) fn validate_delegation_stake(
    mut delegation: Vec<Coin>,
    minimum_delegation: Option<Coin>,
    expected_denom: String,
) -> Result<Coin, MixnetContractError> {
    // check if anything was put as delegation
    if delegation.is_empty() {
        return Err(MixnetContractError::EmptyDelegation);
    }

    if delegation.len() > 1 {
        return Err(MixnetContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if delegation[0].denom != expected_denom {
        return Err(MixnetContractError::WrongDenom {
            received: delegation[0].denom.clone(),
            expected: expected_denom,
        });
    }

    // if we have a minimum set, check if enough tokens were sent, otherwise just check if its non-zero
    if let Some(minimum_delegation) = minimum_delegation {
        return Err(MixnetContractError::InsufficientDelegation {
            received: delegation[0].clone(),
            minimum: minimum_delegation,
        });
    } else if delegation[0].amount.is_zero() {
        return Err(MixnetContractError::EmptyDelegation);
    }

    Ok(delegation.pop().unwrap())
}

pub(crate) fn ensure_is_authorized(
    sender: Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if sender != crate::mixnet_contract_settings::storage::rewarding_validator_address(storage)? {
        return Err(MixnetContractError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn ensure_is_owner(
    sender: Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if sender
        != crate::mixnet_contract_settings::storage::CONTRACT_STATE
            .load(storage)?
            .owner
    {
        return Err(MixnetContractError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn ensure_proxy_match(
    actual: &Option<Addr>,
    expected: &Option<Addr>,
) -> Result<(), MixnetContractError> {
    if actual != expected {
        return Err(MixnetContractError::ProxyMismatch {
            existing: expected
                .as_ref()
                .map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
            incoming: actual
                .as_ref()
                .map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
        });
    }
    Ok(())
}

pub(crate) fn ensure_bonded(bond: &MixNodeBond) -> Result<(), MixnetContractError> {
    if bond.is_unbonding {
        return Err(MixnetContractError::MixnodeIsUnbonding { node_id: bond.id });
    }
    Ok(())
}

// check if the target address has already bonded a mixnode or gateway,
// in either case, return an appropriate error
pub(crate) fn ensure_no_existing_bond(
    storage: &dyn Storage,
    sender: &Addr,
) -> Result<(), MixnetContractError> {
    if mixnodes_storage::mixnode_bonds()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsMixnode);
    }

    if gateways_storage::gateways()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsGateway);
    }

    Ok(())
}

pub(crate) fn validate_node_identity_signature(
    deps: Deps<'_>,
    owner: &Addr,
    signature: String,
    identity: IdentityKeyRef<'_>,
) -> Result<(), MixnetContractError> {
    let owner_bytes = owner.as_bytes();

    let mut identity_bytes = [0u8; 32];
    let mut signature_bytes = [0u8; 64];

    let identity_used_bytes = bs58::decode(identity)
        .into(&mut identity_bytes)
        .map_err(|err| MixnetContractError::MalformedEd25519IdentityKey(err.to_string()))?;
    let signature_used_bytes = bs58::decode(signature)
        .into(&mut signature_bytes)
        .map_err(|err| MixnetContractError::MalformedEd25519Signature(err.to_string()))?;

    if identity_used_bytes != 32 {
        return Err(MixnetContractError::MalformedEd25519IdentityKey(
            "Too few bytes provided for the public key".into(),
        ));
    }

    if signature_used_bytes != 64 {
        return Err(MixnetContractError::MalformedEd25519Signature(
            "Too few bytes provided for the signature".into(),
        ));
    }

    let res = deps
        .api
        .ed25519_verify(owner_bytes, &signature_bytes, &identity_bytes)
        .map_err(cosmwasm_std::StdError::verification_err)?;
    if !res {
        Err(MixnetContractError::InvalidEd25519Signature)
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
            Err(MixnetContractError::MalformedEd25519IdentityKey(
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
            Err(MixnetContractError::MalformedEd25519Signature(
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
            Err(MixnetContractError::MalformedEd25519IdentityKey(
                "Too few bytes provided for the public key".into()
            )),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr1_key1.clone(),
                short_bs58,
            )
        );

        assert_eq!(
            Err(MixnetContractError::MalformedEd25519Signature(
                "Too few bytes provided for the signature".into()
            )),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                short_bs58.into(),
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(MixnetContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr1_key1.clone(),
                &keypair2.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(MixnetContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address1,
                sig_addr2_key1,
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(MixnetContractError::InvalidEd25519Signature),
            validate_node_identity_signature(
                deps.as_ref(),
                &address2,
                sig_addr1_key1.clone(),
                &keypair1.public_key().to_base58_string(),
            )
        );

        assert_eq!(
            Err(MixnetContractError::InvalidEd25519Signature),
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
