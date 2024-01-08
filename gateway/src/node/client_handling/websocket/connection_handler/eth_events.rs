// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::authenticated::RequestHandlingError;
use nym_bandwidth_claim_contract::msg::ExecuteMsg;
use nym_bandwidth_claim_contract::payment::LinkPaymentData;
use nym_credentials::token::bandwidth::TokenCredential;
use nym_crypto::asymmetric::identity::{PublicKey, Signature, SIGNATURE_LENGTH};
use nym_network_defaults::{ETH_EVENT_NAME, ETH_MIN_BLOCK_DEPTH};
use nym_validator_client::nxmd::traits::MixnetQueryClient;
use nym_validator_client::nyxd::{AccountId, NyxdClient, SigningNyxdClient};
use std::str::FromStr;
use web3::contract::tokens::Detokenize;
use web3::contract::{Contract, Error};
use web3::transports::Http;
use web3::types::{BlockNumber, FilterBuilder, H256};
use web3::Web3;

#[derive(Debug)]
pub struct Burned {
    /// The bandwidth bought by the client
    pub bandwidth: u64,
    /// Client public verification key
    pub verification_key: PublicKey,
    /// Signed verification key
    pub signed_verification_key: Signature,
    /// Address for the owner of the gateway
    pub cosmos_recipient: String,
}

impl Burned {
    pub fn verify(&self, verification_key: PublicKey) -> bool {
        self.verification_key == verification_key
            && verification_key
                .verify(
                    &self.verification_key.to_bytes(),
                    &self.signed_verification_key,
                )
                .is_ok()
    }
}

impl Detokenize for Burned {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if tokens.len() != 4 {
            return Err(Error::InvalidOutputType(format!(
                "Expected three elements, got: {:?}",
                tokens
            )));
        }
        let bandwidth = tokens
            .get(0)
            .unwrap()
            .clone()
            .into_uint()
            .ok_or_else(|| Error::InvalidOutputType(String::from("Expected Uint for bandwidth")))?
            .as_u64();
        let verification_key: [u8; 32] = tokens
            .get(1)
            .unwrap()
            .clone()
            .into_uint()
            .ok_or_else(|| {
                Error::InvalidOutputType(String::from("Expected Uint for verification key"))
            })?
            .into();
        let verification_key = PublicKey::from_bytes(&verification_key).map_err(|_| {
            Error::InvalidOutputType(format!(
                "Expected verification key of 32 bytes, got: {}",
                verification_key.len()
            ))
        })?;
        let signed_verification_key =
            tokens.get(2).unwrap().clone().into_bytes().ok_or_else(|| {
                Error::InvalidOutputType(String::from("Expected Bytes for the last two fields"))
            })?;
        let signed_verification_key =
            Signature::from_bytes(&signed_verification_key[..SIGNATURE_LENGTH]).map_err(|_| {
                Error::InvalidOutputType(format!(
                    "Expected signature of {} bytes, got: {}",
                    SIGNATURE_LENGTH,
                    signed_verification_key.len()
                ))
            })?;
        let cosmos_recipient = tokens
            .get(3)
            .unwrap()
            .clone()
            .into_string()
            .ok_or_else(|| {
                Error::InvalidOutputType(String::from("Expected utf8 encoded owner address"))
            })?;

        Ok(Burned {
            bandwidth,
            verification_key,
            signed_verification_key,
            cosmos_recipient,
        })
    }
}
