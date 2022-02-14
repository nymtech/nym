// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bip39::core::str::FromStr;
use bip39::Mnemonic;
use rand::seq::SliceRandom;
use rand::thread_rng;
use url::Url;
use web3::contract::tokens::Detokenize;
use web3::contract::{Contract, Error};
use web3::ethabi::Token;
use web3::transports::Http;
use web3::types::{BlockNumber, FilterBuilder, H256};
use web3::Web3;

use crate::node::client_handling::websocket::connection_handler::authenticated::RequestHandlingError;
use bandwidth_claim_contract::msg::ExecuteMsg;
use bandwidth_claim_contract::payment::LinkPaymentData;
use credentials::token::bandwidth::TokenCredential;
use crypto::asymmetric::identity::{PublicKey, Signature, SIGNATURE_LENGTH};
use gateway_client::bandwidth::eth_contract;
use network_defaults::{
    DEFAULT_BANDWIDTH_CLAIM_CONTRACT_ADDRESS, DEFAULT_MIXNET_CONTRACT_ADDRESS, ETH_EVENT_NAME,
    ETH_MIN_BLOCK_DEPTH,
};
use validator_client::nymd::{AccountId, NymdClient, SigningNymdClient};

pub(crate) struct ERC20Bridge {
    // This is needed because web3's Contract doesn't sufficiently expose it's eth interface
    web3: Web3<Http>,
    contract: Contract<Http>,
    nymd_client: NymdClient<SigningNymdClient>,
}

impl ERC20Bridge {
    pub fn new(eth_endpoint: String, nymd_urls: Vec<Url>, cosmos_mnemonic: String) -> Self {
        let transport = Http::new(&eth_endpoint).expect("Invalid Ethereum endpoint");
        let web3 = Web3::new(transport);
        let nymd_url = nymd_urls
            .choose(&mut thread_rng())
            .expect("The list of validators is empty");
        let mnemonic =
            Mnemonic::from_str(&cosmos_mnemonic).expect("Invalid Cosmos mnemonic provided");
        let nymd_client = NymdClient::connect_with_mnemonic(
            nymd_url.as_ref(),
            AccountId::from_str(DEFAULT_MIXNET_CONTRACT_ADDRESS).ok(),
            None,
            AccountId::from_str(DEFAULT_BANDWIDTH_CLAIM_CONTRACT_ADDRESS).ok(),
            mnemonic,
            None,
        )
        .expect("Could not create nymd client");

        ERC20Bridge {
            contract: eth_contract(web3.clone()),
            web3,
            nymd_client,
        }
    }

    pub(crate) async fn verify_eth_events(
        &self,
        verification_key: PublicKey,
    ) -> Result<String, RequestHandlingError> {
        // It's safe to unwrap here, as we are guarded by a unit test that checks the event
        // name constant against the contract abi
        let event = self.contract.abi().event(ETH_EVENT_NAME).unwrap();
        let latest_block = self.web3.eth().block_number().await?;
        let check_until = if cfg!(debug_assertions) {
            latest_block
        } else {
            latest_block - ETH_MIN_BLOCK_DEPTH
        };
        let filter = FilterBuilder::default()
            .address(vec![self.contract.address()])
            .topics(
                Some(vec![event.signature()]),
                Some(vec![H256::from(verification_key.to_bytes())]),
                None,
                None,
            )
            .from_block(BlockNumber::Earliest)
            .to_block(BlockNumber::Number(check_until))
            .build();
        // Get only the first event that checks out. If the client burns more tokens with the
        // same verification key, those tokens would be lost
        for l in self.web3.eth().logs(filter).await? {
            let log = event.parse_log(web3::ethabi::RawLog {
                topics: l.topics,
                data: l.data.0,
            })?;
            let burned_event =
                Burned::from_tokens(log.params.into_iter().map(|x| x.value).collect::<Vec<_>>())?;
            if burned_event.verify(verification_key) {
                return Ok(burned_event.cosmos_recipient);
            }
        }

        Err(RequestHandlingError::InvalidBandwidthCredential)
    }

    pub(crate) async fn verify_gateway_owner(
        &self,
        gateway_owner: String,
        gateway_identity: &PublicKey,
    ) -> Result<(), RequestHandlingError> {
        let owner_address = AccountId::from_str(&gateway_owner)
            .map_err(|_| RequestHandlingError::InvalidBandwidthCredential)?;
        let gateway_bond = self
            .nymd_client
            .owns_gateway(&owner_address)
            .await?
            .ok_or(RequestHandlingError::InvalidBandwidthCredential)?;
        if gateway_bond.gateway.identity_key == gateway_identity.to_base58_string() {
            Ok(())
        } else {
            Err(RequestHandlingError::InvalidBandwidthCredential)
        }
    }

    pub(crate) async fn claim_token(
        &self,
        credential: &TokenCredential,
    ) -> Result<(), RequestHandlingError> {
        // It's ok to unwrap here, as the cosmos contract is set correctly
        let erc20_bridge_contract_address =
            self.nymd_client.erc20_bridge_contract_address().unwrap();
        let req = ExecuteMsg::LinkPayment {
            data: LinkPaymentData::new(
                credential.verification_key().to_bytes(),
                credential.gateway_identity().to_bytes(),
                credential.bandwidth(),
                credential.signature_bytes(),
            ),
        };
        self.nymd_client
            .execute(
                erc20_bridge_contract_address,
                &req,
                Default::default(),
                "Linking payment",
                vec![],
            )
            .await?;
        Ok(())
    }
}

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
