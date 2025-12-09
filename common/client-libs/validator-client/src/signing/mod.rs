// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::direct_wallet::DirectSecp256k1HdWalletError;
use bip32::XPrv;
use cosmrs::bip32::DerivationPath;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::crypto::PublicKey;
use cosmrs::tendermint::chain;
use cosmrs::tx::{AccountNumber, SequenceNumber};
use cosmrs::AccountId;

pub mod direct_wallet;
pub mod signer;
pub mod tx_signer;

pub(crate) type Secp256k1Keypair = (SigningKey, PublicKey);

/// Derivation information required to derive a keypair and an address from a mnemonic.
#[derive(Debug, Clone)]
pub(crate) struct Secp256k1Derivation {
    hd_path: DerivationPath,
    prefix: String,
}

impl Secp256k1Derivation {
    pub(crate) fn try_derive_account<S>(
        &self,
        seed: S,
    ) -> Result<AccountData, DirectSecp256k1HdWalletError>
    where
        S: AsRef<[u8]>,
    {
        let keypair = derive_keypair(seed, &self.hd_path)?;

        // it seems this can only fail if the provided account prefix is invalid
        let address = keypair
            .1
            .account_id(&self.prefix)
            .map_err(|source| DirectSecp256k1HdWalletError::AccountDerivationError { source })?;

        Ok(AccountData {
            address,
            public_key: keypair.1,
            private_key: keypair.0,
        })
    }
}

pub fn derive_keypair<S>(
    seed: S,
    hd_path: &DerivationPath,
) -> Result<Secp256k1Keypair, DirectSecp256k1HdWalletError>
where
    S: AsRef<[u8]>,
{
    let extended_private_key = derive_extended_private_key(seed, hd_path)?;

    let private_key: SigningKey = extended_private_key.into();
    let public_key = private_key.public_key();

    Ok((private_key, public_key))
}

pub fn derive_extended_private_key<S>(
    seed: S,
    hd_path: &DerivationPath,
) -> Result<XPrv, DirectSecp256k1HdWalletError>
where
    S: AsRef<[u8]>,
{
    Ok(XPrv::derive_from_path(seed, hd_path)?)
}

pub struct AccountData {
    pub address: AccountId,

    pub(crate) public_key: PublicKey,

    pub(crate) private_key: SigningKey,
}

impl AccountData {
    pub fn address(&self) -> &AccountId {
        &self.address
    }

    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    pub fn private_key(&self) -> &SigningKey {
        &self.private_key
    }
}

/// Signing information for a single signer that is not included in the transaction.
#[derive(Debug)]
pub struct SignerData {
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
    pub chain_id: chain::Id,
}

impl SignerData {
    pub fn new(
        account_number: AccountNumber,
        sequence: SequenceNumber,
        chain_id: chain::Id,
    ) -> Self {
        SignerData {
            account_number,
            sequence,
            chain_id,
        }
    }

    pub fn new_from_sequence_response(
        response: crate::nyxd::cosmwasm_client::types::SequenceResponse,
        chain_id: chain::Id,
    ) -> Self {
        SignerData {
            account_number: response.account_number,
            sequence: response.sequence,
            chain_id,
        }
    }
}
