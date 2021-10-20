// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity::PublicKey;
use gateway_client::bandwidth::eth_contract;
use web3::contract::tokens::Detokenize;
use web3::contract::{Contract, Error};
use web3::ethabi::{Bytes, Token, Uint};
use web3::transports::Http;
use web3::Web3;

#[derive(Clone)]
pub struct EthEvents {
    // This is needed because web3's Contract doesn't sufficiently expose it's eth interface
    web3: Web3<Http>,
    contract: Contract<Http>,
}

impl EthEvents {
    pub fn new(web3: Web3<Http>) -> Self {
        EthEvents {
            contract: eth_contract(web3.clone()),
            web3,
        }
    }

    pub async fn verify_eth_events(&self, _public_key: PublicKey) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Burned {
    /// The bandwidth bought by the client
    pub bandwidth: Uint,
    /// Client public verification key
    pub verification_key: Uint,
    /// Signed verification key
    pub signed_verification_key: Bytes,
}

impl Detokenize for Burned {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if tokens.len() != 3 {
            return Err(Error::InvalidOutputType(format!(
                "Expected three elements, got: {:?}",
                tokens
            )));
        }
        let bandwidth =
            tokens.get(0).unwrap().clone().into_uint().ok_or_else(|| {
                Error::InvalidOutputType(String::from("Expected Uint for bandwidth"))
            })?;
        let verification_key = tokens.get(1).unwrap().clone().into_uint().ok_or_else(|| {
            Error::InvalidOutputType(String::from("Expected Uint for verification key"))
        })?;
        let signed_verification_key =
            tokens.get(2).unwrap().clone().into_bytes().ok_or_else(|| {
                Error::InvalidOutputType(String::from("Expected Bytes for signed_verification_key"))
            })?;

        Ok(Burned {
            bandwidth,
            verification_key,
            signed_verification_key,
        })
    }
}
