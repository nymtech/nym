// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::helpers::CheckResponse;
use crate::nymd::cosmwasm_client::logs::{self, parse_raw_logs};
use crate::nymd::cosmwasm_client::types::*;
use crate::nymd::cosmwasm_client::HttpClient;
use crate::nymd::cosmwasm_client::{DEFAULT_BROADCAST_POLLING_RATE, DEFAULT_BROADCAST_TIMEOUT};
use crate::nymd::fee::GasAdjustable;
use crate::nymd::wallet_client::rpc::SimpleRequest;
use crate::nymd::DEFAULT_SIMULATED_GAS_MULTIPLIER;
use crate::nymd::{
    DirectSecp256k1HdWallet, GasPrice, HttpClientUrl, NymdError, SigningCosmWasmClient,
    SimulateResponse, TendermintRpcError, TxResponse,
};
use async_trait::async_trait;
use cosmrs::proto::cosmos::tx::signing::v1beta1::SignMode;
use cosmrs::rpc;
use cosmrs::tx::mode_info::Single;
use cosmrs::tx::{self, Gas, ModeInfo, Msg, MsgProto, SignDoc, SignerInfo};
use cosmrs::{AccountId, Any};
use ledger::CosmosLedger;
use log::{debug, info};
use mixnet_contract_common::ExecuteMsg;
use serde::Serialize;
use std::str::FromStr;
use std::time::Duration;

fn single_unspecified_signer_auth(
    public_key: Option<tx::SignerPublicKey>,
    sequence_number: tx::SequenceNumber,
) -> tx::AuthInfo {
    tx::SignerInfo {
        public_key,
        mode_info: tx::ModeInfo::Single(tx::mode_info::Single {
            mode: SignMode::Unspecified,
        }),
        sequence: sequence_number,
    }
    .auth_info(empty_fee())
}

fn empty_fee() -> tx::Fee {
    tx::Fee {
        amount: vec![],
        gas_limit: Default::default(),
        payer: None,
        granter: None,
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    rpc_client: HttpClient,
    signer: CosmosLedger,
    gas_price: GasPrice,

    broadcast_polling_rate: Duration,
    broadcast_timeout: Duration,
}

impl Client {
    pub fn connect_with_ledger<U: Clone>(
        endpoint: U,
        signer: CosmosLedger,
        gas_price: GasPrice,
    ) -> Result<Self, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let rpc_client = HttpClient::new(endpoint)?;
        Ok(Client {
            rpc_client,
            signer,
            gas_price,
            broadcast_polling_rate: DEFAULT_BROADCAST_POLLING_RATE,
            broadcast_timeout: DEFAULT_BROADCAST_TIMEOUT,
        })
    }

    pub fn set_broadcast_polling_rate(&mut self, broadcast_polling_rate: Duration) {
        self.broadcast_polling_rate = broadcast_polling_rate
    }

    pub fn set_broadcast_timeout(&mut self, broadcast_timeout: Duration) {
        self.broadcast_timeout = broadcast_timeout
    }

    pub fn signer(&self) -> CosmosLedger {
        self.signer.clone()
    }
}

#[async_trait]
impl rpc::Client for Client {
    async fn perform<R>(&self, request: R) -> Result<R::Response, rpc::Error>
    where
        R: SimpleRequest,
    {
        self.rpc_client.perform(request).await
    }
}

#[async_trait]
impl CosmWasmClient for Client {
    fn broadcast_polling_rate(&self) -> Duration {
        self.broadcast_polling_rate
    }

    fn broadcast_timeout(&self) -> Duration {
        self.broadcast_timeout
    }
}

impl MinimalSigningCosmWasmClient for Client {
    fn signer(&self) -> CosmosLedger {
        self.signer.clone()
    }

    fn gas_price(&self) -> &GasPrice {
        &self.gas_price
    }
}

impl SigningCosmWasmClient for Client {
    fn signer(&self) -> &DirectSecp256k1HdWallet {
        todo!()
    }

    fn gas_price(&self) -> &GasPrice {
        &self.gas_price
    }
}

#[derive(Debug, Serialize)]
struct Coin {
    amount: String,
    denom: String,
}

impl From<crate::nymd::Coin> for Coin {
    fn from(coin: crate::nymd::Coin) -> Self {
        Coin {
            amount: coin.amount.to_string(),
            denom: coin.denom.to_string(),
        }
    }
}

impl From<cosmrs::Coin> for Coin {
    fn from(coin: cosmrs::Coin) -> Self {
        let nymd_coin: crate::nymd::Coin = coin.into();
        nymd_coin.into()
    }
}

#[derive(Debug, Serialize)]
struct Fee {
    amount: Vec<Coin>,
    gas: String,
}

#[derive(Debug, Serialize)]
struct SendValue {
    amount: Vec<Coin>,
    from_address: String,
    to_address: String,
}

#[derive(Debug, Serialize)]
struct MsgSend {
    #[serde(rename = "type")]
    type_url: String,
    value: SendValue,
}

#[derive(Debug, Serialize)]
struct SendTransaction {
    account_number: String,
    chain_id: String,
    fee: Fee,
    memo: String,
    msgs: Vec<MsgSend>,
    sequence: String,
}

#[derive(Serialize)]
struct ExecuteContractValue {
    contract: String,
    funds: Vec<Coin>,
    msg: ExecuteMsg,
    sender: String,
}

#[derive(Serialize)]
struct MsgExecuteContract {
    #[serde(rename = "type")]
    type_url: String,
    value: ExecuteContractValue,
}

#[derive(Serialize)]
struct ExecuteContractTransaction {
    account_number: String,
    chain_id: String,
    fee: Fee,
    memo: String,
    msgs: Vec<MsgExecuteContract>,
    sequence: String,
}

#[async_trait]
pub trait MinimalSigningCosmWasmClient: CosmWasmClient {
    fn signer(&self) -> CosmosLedger;

    fn gas_price(&self) -> &GasPrice;

    fn signer_public_key(&self, signer_address: &AccountId) -> Option<tx::SignerPublicKey> {
        let response = self.signer().get_addr_secp265k1(false).ok()?;
        if response.address == signer_address.to_string() {
            let verifying_key: cosmrs::crypto::secp256k1::VerifyingKey = response.public_key.into();
            let cosmrs_public_key: cosmrs::crypto::PublicKey = verifying_key.into();
            Some(cosmrs_public_key.into())
        } else {
            None
        }
    }

    async fn simulate(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<SimulateResponse, NymdError> {
        let public_key = self.signer_public_key(signer_address);
        let sequence_response = self.get_sequence(signer_address).await?;

        let partial_tx = cosmrs::Tx {
            body: tx::Body::new(messages, memo, 0u32),
            auth_info: single_unspecified_signer_auth(public_key, sequence_response.sequence),
            signatures: vec![Vec::new()],
        };
        self.query_simulate(Some(partial_tx), Vec::new()).await

        // for completion sake, once we're able to transition into using `tx_bytes`,
        // we might want to use something like this instead:
        // let tx_raw: tx::Raw = cosmrs::proto::cosmos::tx::v1beta1::TxRaw {
        //     body_bytes: partial_tx.body.into_bytes().unwrap(),
        //     auth_info_bytes: partial_tx.auth_info.into_bytes().unwrap(),
        //     signatures: partial_tx.signatures,
        // }
        // .into();
        // self.query_simulate(None, tx_raw.to_bytes().unwrap()).await
    }

    // in this particular case we cannot generalise the argument to `&str` due to lifetime constraints
    #[allow(clippy::ptr_arg)]
    async fn determine_transaction_fee(
        &self,
        signer_address: &AccountId,
        messages: &[Any],
        fee: crate::nymd::fee::Fee,
        memo: &String,
    ) -> Result<tx::Fee, NymdError> {
        let auto_fee = |multiplier: Option<f32>| async move {
            debug!("Trying to simulate gas costs...");
            // from what I've seen in manual testing, gas estimation does not exist if transaction
            // fails to get executed (for example if you send 'BondMixnode" with invalid signature)
            let gas_estimation = self
                .simulate(signer_address, messages.to_vec(), memo.clone())
                .await?
                .gas_info
                .ok_or(NymdError::GasEstimationFailure)?
                .gas_used;

            let multiplier = multiplier.unwrap_or(DEFAULT_SIMULATED_GAS_MULTIPLIER);
            let gas = gas_estimation.adjust_gas(multiplier);

            debug!("Gas estimation: {}", gas_estimation);
            debug!("Multiplying the estimation by {}", multiplier);
            debug!("Final gas limit used: {}", gas);

            let fee = self.gas_price() * gas;
            Ok::<tx::Fee, NymdError>(tx::Fee::from_amount_and_gas(fee, gas))
        };
        let fee = match fee {
            crate::nymd::fee::Fee::Manual(fee) => fee,
            crate::nymd::fee::Fee::Auto(multiplier) => auto_fee(multiplier).await?,
            crate::nymd::fee::Fee::PayerGranterAuto(auto_feegrant) => {
                let mut fee = auto_fee(auto_feegrant.gas_adjustment).await?;
                fee.payer = auto_feegrant.payer;
                fee.granter = auto_feegrant.granter;
                fee
            }
        };
        debug!("Fee used for the transaction: {:?}", fee);
        Ok(fee)
    }

    fn sign(
        &self,
        messages: String,
        cosmrs_messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, NymdError> {
        let signature = self
            .signer()
            .sign_secp265k1(messages)
            .expect("Could not sign")
            .signature;
        let response = self.signer().get_addr_secp265k1(false)?;
        let verifying_key: cosmrs::crypto::secp256k1::VerifyingKey = response.public_key.into();
        let cosmrs_public_key: cosmrs::crypto::PublicKey = verifying_key.into();

        // TODO: WTF HOW IS TIMEOUT_HEIGHT SUPPOSED TO GET DETERMINED?
        // IT DOESNT EXIST IN COSMJS!!
        // try to set to 0
        let timeout_height = 0u32;

        let tx_body = tx::Body::new(cosmrs_messages, memo, timeout_height);
        let mut signer_info =
            SignerInfo::single_direct(Some(cosmrs_public_key), signer_data.sequence);
        signer_info.mode_info = ModeInfo::Single(Single {
            mode: SignMode::LegacyAminoJson,
        });
        let auth_info = signer_info.auth_info(fee);

        // ideally I'd prefer to have the entire error put into the NymdError::SigningFailure
        // but I'm super hesitant to trying to downcast the eyre::Report to cosmrs::error::Error
        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &signer_data.chain_id,
            signer_data.account_number,
        )
        .map_err(|_| NymdError::SigningFailure)?;

        Ok(cosmrs::proto::cosmos::tx::v1beta1::TxRaw {
            body_bytes: sign_doc.body_bytes,
            auth_info_bytes: sign_doc.auth_info_bytes,
            signatures: vec![signature.as_ref().to_vec()],
        }
        .into())
    }

    async fn send_tokens(
        &self,
        sender_address: &AccountId,
        recipient_address: &AccountId,
        amount: Vec<crate::nymd::Coin>,
        fee: crate::nymd::fee::Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let cosmrs_msg = cosmrs::bank::MsgSend {
            from_address: sender_address.clone(),
            to_address: recipient_address.clone(),
            amount: amount.iter().cloned().map(Into::into).collect(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgSend".to_owned()))?;
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(sender_address, &vec![cosmrs_msg.clone()], fee, &memo)
            .await?;

        let response = self.signer().get_addr_secp265k1(false)?;
        let sequence_response = self
            .get_sequence(&AccountId::from_str(&response.address).unwrap())
            .await?;
        let chain_id = self.get_chain_id().await?;

        let signer_data = SignerData {
            account_number: sequence_response.account_number,
            sequence: sequence_response.sequence,
            chain_id,
        };

        let send_msg = MsgSend {
            type_url: String::from("cosmos-sdk/MsgSend"),
            value: SendValue {
                from_address: sender_address.to_string(),
                to_address: recipient_address.to_string(),
                amount: amount.iter().cloned().map(Into::into).collect(),
            },
        };
        let tx = SendTransaction {
            account_number: signer_data.account_number.to_string(),
            chain_id: signer_data.chain_id.to_string(),
            fee: Fee {
                amount: fee.amount.iter().cloned().map(Into::into).collect(),
                gas: fee.gas_limit.to_string(),
            },
            memo: memo.clone(),
            msgs: vec![send_msg],
            sequence: signer_data.sequence.to_string(),
        };
        let tx_bytes = self
            .sign(
                serde_json::to_string(&tx).unwrap(),
                vec![cosmrs_msg],
                fee,
                memo,
                signer_data,
            )?
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        self.broadcast_tx(tx_bytes.into()).await
    }

    async fn wallet_execute(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        msg: &ExecuteMsg,
        fee: crate::nymd::fee::Fee,
        memo: impl Into<String> + Send + 'static,
        funds: Vec<crate::nymd::Coin>,
    ) -> Result<ExecuteResult, NymdError> {
        let cosmrs_msg = cosmrs::cosmwasm::MsgExecuteContract {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
            msg: serde_json::to_vec(msg)?,
            funds: funds.iter().cloned().map(Into::into).collect(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgExecuteContract".to_owned()))?;
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(sender_address, &vec![cosmrs_msg.clone()], fee, &memo)
            .await?;

        let response = self.signer().get_addr_secp265k1(false)?;
        let sequence_response = self
            .get_sequence(&AccountId::from_str(&response.address).unwrap())
            .await?;
        let chain_id = self.get_chain_id().await?;

        let signer_data = SignerData {
            account_number: sequence_response.account_number,
            sequence: sequence_response.sequence,
            chain_id,
        };

        let execute_contract_msg = MsgExecuteContract {
            type_url: String::from("wasm/MsgExecuteContract"),
            value: ExecuteContractValue {
                contract: contract_address.to_string(),
                funds: funds.iter().cloned().map(Into::into).collect(),
                msg: msg.clone(),
                sender: sender_address.to_string(),
            },
        };
        let tx = ExecuteContractTransaction {
            account_number: signer_data.account_number.to_string(),
            chain_id: signer_data.chain_id.to_string(),
            fee: Fee {
                amount: fee.amount.iter().cloned().map(Into::into).collect(),
                gas: fee.gas_limit.to_string(),
            },
            memo: memo.clone(),
            msgs: vec![execute_contract_msg],
            sequence: signer_data.sequence.to_string(),
        };
        let tx_bytes = self
            .sign(
                serde_json::to_string(&tx).unwrap(),
                vec![cosmrs_msg],
                fee,
                memo,
                signer_data,
            )?
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        let tx_res = self.broadcast_tx(tx_bytes.into()).await?.check_response()?;

        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            data: tx_res.tx_result.data,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }
}
