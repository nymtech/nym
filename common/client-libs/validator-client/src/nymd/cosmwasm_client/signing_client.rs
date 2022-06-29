// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::helpers::{compress_wasm_code, CheckResponse};
use crate::nymd::cosmwasm_client::logs::{self, parse_raw_logs};
use crate::nymd::cosmwasm_client::types::*;
use crate::nymd::error::NymdError;
use crate::nymd::fee::{Fee, DEFAULT_SIMULATED_GAS_MULTIPLIER};
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use crate::nymd::{Coin, GasAdjustable, GasPrice, TxResponse};
use async_trait::async_trait;
use cosmrs::bank::MsgSend;
use cosmrs::distribution::MsgWithdrawDelegatorReward;
use cosmrs::feegrant::{
    AllowedMsgAllowance, BasicAllowance, MsgGrantAllowance, MsgRevokeAllowance,
};
use cosmrs::proto::cosmos::tx::signing::v1beta1::SignMode;
use cosmrs::rpc::endpoint::broadcast;
use cosmrs::rpc::{Error as TendermintRpcError, HttpClient, HttpClientUrl, SimpleRequest};
use cosmrs::staking::{MsgDelegate, MsgUndelegate};
use cosmrs::tx::{self, Msg, SignDoc, SignerInfo};
use cosmrs::{cosmwasm, rpc, AccountId, Any, Tx};
use log::debug;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::convert::TryInto;
use std::time::{Duration, SystemTime};

const DEFAULT_BROADCAST_POLLING_RATE: Duration = Duration::from_secs(4);
const DEFAULT_BROADCAST_TIMEOUT: Duration = Duration::from_secs(60);

fn empty_fee() -> tx::Fee {
    tx::Fee {
        amount: vec![],
        gas_limit: Default::default(),
        payer: None,
        granter: None,
    }
}

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

#[async_trait]
pub trait SigningCosmWasmClient: CosmWasmClient {
    fn signer(&self) -> &DirectSecp256k1HdWallet;

    fn gas_price(&self) -> &GasPrice;

    fn signer_public_key(&self, signer_address: &AccountId) -> Option<tx::SignerPublicKey> {
        let signer_accounts = self.signer().try_derive_accounts().ok()?;
        let account_from_signer = signer_accounts
            .iter()
            .find(|account| &account.address == signer_address)?;
        let public_key = account_from_signer.public_key;
        Some(public_key.into())
    }

    async fn simulate(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<SimulateResponse, NymdError> {
        let public_key = self.signer_public_key(signer_address);
        let sequence_response = self.get_sequence(signer_address).await?;

        let partial_tx = Tx {
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

    async fn upload(
        &self,
        sender_address: &AccountId,
        wasm_code: Vec<u8>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<UploadResult, NymdError> {
        let compressed = compress_wasm_code(&wasm_code)?;
        let compressed_size = compressed.len();
        let compressed_checksum = Sha256::digest(&compressed).to_vec();

        // TODO: what about instantiate_permission?
        // cosmjs is just ignoring that field...
        let upload_msg = cosmwasm::MsgStoreCode {
            sender: sender_address.clone(),
            wasm_byte_code: compressed,
            instantiate_permission: Default::default(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgStoreCode".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![upload_msg], fee, memo)
            .await?
            .check_response()?;

        let logs = parse_raw_logs(tx_res.tx_result.log)?;
        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        // TODO: should those strings be extracted into some constants?
        // the reason I think unwrap here is fine is that if the transaction succeeded and those
        // fields do not exist or code_id is not a number, there's no way we can recover, we're probably connected
        // to wrong validator or something
        let code_id = logs::find_attribute(&logs, "store_code", "code_id")
            .unwrap()
            .value
            .parse()
            .unwrap();

        Ok(UploadResult {
            original_size: wasm_code.len(),
            original_checksum: Sha256::digest(&wasm_code).to_vec(),
            compressed_size,
            compressed_checksum,
            code_id,
            logs,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    // honestly, I don't see a nice way of removing any arguments
    // perhaps memo could be moved to options like what cosmjs is doing
    // put personally I'd prefer to leave it there for consistency with
    // signatures of other methods
    #[allow(clippy::too_many_arguments)]
    async fn instantiate<M>(
        &self,
        sender_address: &AccountId,
        code_id: ContractCodeId,
        msg: &M,
        label: String,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
        mut options: Option<InstantiateOptions>,
    ) -> Result<InstantiateResult, NymdError>
    where
        M: ?Sized + Serialize + Sync,
    {
        let init_msg = cosmwasm::MsgInstantiateContract {
            sender: sender_address.clone(),
            admin: options.as_mut().and_then(|options| options.admin.take()),
            code_id,
            // now this is a weird one. the protobuf files say this field is optional,
            // but if you omit it, the initialisation will fail CheckTx
            label: Some(label),
            msg: serde_json::to_vec(msg)?,
            funds: options.map(|options| options.funds).unwrap_or_default(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgInstantiateContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![init_msg], fee, memo)
            .await?
            .check_response()?;

        let logs = parse_raw_logs(tx_res.tx_result.log)?;
        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        // TODO: should those strings be extracted into some constants?
        // the reason I think unwrap here is fine is that if the transaction succeeded and those
        // fields do not exist or address is malformed, there's no way we can recover, we're probably connected
        // to wrong validator or something
        let contract_address = logs::find_attribute(&logs, "instantiate", "_contract_address")
            .unwrap()
            .value
            .parse()
            .unwrap();

        Ok(InstantiateResult {
            contract_address,
            logs,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    async fn update_admin(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        new_admin: &AccountId,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ChangeAdminResult, NymdError> {
        let change_admin_msg = cosmwasm::MsgUpdateAdmin {
            sender: sender_address.clone(),
            new_admin: new_admin.clone(),
            contract: contract_address.clone(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgUpdateAdmin".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        Ok(ChangeAdminResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    async fn clear_admin(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ChangeAdminResult, NymdError> {
        let change_admin_msg = cosmwasm::MsgClearAdmin {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgClearAdmin".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        Ok(ChangeAdminResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    async fn migrate<M>(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        code_id: u64,
        fee: Fee,
        msg: &M,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<MigrateResult, NymdError>
    where
        M: ?Sized + Serialize + Sync,
    {
        let migrate_msg = cosmwasm::MsgMigrateContract {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
            code_id,
            msg: serde_json::to_vec(msg)?,
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgMigrateContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![migrate_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        Ok(MigrateResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    async fn execute<M>(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        msg: &M,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NymdError>
    where
        M: ?Sized + Serialize + Sync,
    {
        let execute_msg = cosmwasm::MsgExecuteContract {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
            msg: serde_json::to_vec(msg)?,
            funds: funds.into_iter().map(Into::into).collect(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgExecuteContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![execute_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            data: tx_res.tx_result.data,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    async fn execute_multiple<I, M>(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        msgs: I,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ExecuteResult, NymdError>
    where
        I: IntoIterator<Item = (M, Vec<Coin>)> + Send,
        M: Serialize,
    {
        let messages = msgs
            .into_iter()
            .map(|(msg, funds)| {
                cosmwasm::MsgExecuteContract {
                    sender: sender_address.clone(),
                    contract: contract_address.clone(),
                    msg: serde_json::to_vec(&msg)?,
                    funds: funds.into_iter().map(Into::into).collect(),
                }
                .to_any()
                .map_err(|_| NymdError::SerializationError("MsgExecuteContract".to_owned()))
            })
            .collect::<Result<_, _>>()?;

        let tx_res = self
            .sign_and_broadcast(sender_address, messages, fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo::new(tx_res.tx_result.gas_wanted, tx_res.tx_result.gas_used);

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            data: tx_res.tx_result.data,
            transaction_hash: tx_res.hash,
            gas_info,
        })
    }

    async fn send_tokens(
        &self,
        sender_address: &AccountId,
        recipient_address: &AccountId,
        amount: Vec<Coin>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let send_msg = MsgSend {
            from_address: sender_address.clone(),
            to_address: recipient_address.clone(),
            amount: amount.into_iter().map(Into::into).collect(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgSend".to_owned()))?;

        self.sign_and_broadcast(sender_address, vec![send_msg], fee, memo)
            .await?
            .check_response()
    }

    async fn send_tokens_multiple<I>(
        &self,
        sender_address: &AccountId,
        msgs: I,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError>
    where
        I: IntoIterator<Item = (AccountId, Vec<Coin>)> + Send,
    {
        let messages = msgs
            .into_iter()
            .map(|(to_address, amount)| {
                MsgSend {
                    from_address: sender_address.clone(),
                    to_address,
                    amount: amount.into_iter().map(Into::into).collect(),
                }
                .to_any()
                .map_err(|_| NymdError::SerializationError("MsgExecuteContract".to_owned()))
            })
            .collect::<Result<_, _>>()?;

        self.sign_and_broadcast(sender_address, messages, fee, memo)
            .await?
            .check_response()
    }

    async fn grant_allowance(
        &self,
        granter: &AccountId,
        grantee: &AccountId,
        spend_limit: Vec<Coin>,
        expiration: Option<SystemTime>,
        allowed_messages: Vec<String>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let basic_allowance = BasicAllowance {
            spend_limit: spend_limit.into_iter().map(Into::into).collect(),
            expiration,
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("BasicAllowance".to_owned()))?;

        let allowed_msg_allowance = AllowedMsgAllowance {
            allowance: Some(basic_allowance),
            allowed_messages,
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("AllowedMsgAllowance".to_owned()))?;

        let grant_allowance_msg = MsgGrantAllowance {
            granter: granter.to_owned(),
            grantee: grantee.to_owned(),
            allowance: Some(allowed_msg_allowance),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgGrantAllowance".to_owned()))?;

        self.sign_and_broadcast(granter, vec![grant_allowance_msg], fee, memo)
            .await?
            .check_response()
    }

    async fn revoke_allowance(
        &self,
        granter: &AccountId,
        grantee: &AccountId,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let revoke_allowance_msg = MsgRevokeAllowance {
            granter: granter.to_owned(),
            grantee: grantee.to_owned(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgRevokeAllowance".to_owned()))?;

        self.sign_and_broadcast(granter, vec![revoke_allowance_msg], fee, memo)
            .await?
            .check_response()
    }

    async fn delegate_tokens(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let delegate_msg = MsgDelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: amount.into(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgDelegate".to_owned()))?;

        self.sign_and_broadcast(delegator_address, vec![delegate_msg], fee, memo)
            .await?
            .check_response()
    }

    async fn undelegate_tokens(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let undelegate_msg = MsgUndelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: amount.into(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgUndelegate".to_owned()))?;

        self.sign_and_broadcast(delegator_address, vec![undelegate_msg], fee, memo)
            .await?
            .check_response()
    }

    async fn withdraw_rewards(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let withdraw_msg = MsgWithdrawDelegatorReward {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgWithdrawDelegatorReward".to_owned()))?;

        self.sign_and_broadcast(delegator_address, vec![withdraw_msg], fee, memo)
            .await?
            .check_response()
    }

    // in this particular case we cannot generalise the argument to `&str` due to lifetime constraints
    #[allow(clippy::ptr_arg)]
    async fn determine_transaction_fee(
        &self,
        signer_address: &AccountId,
        messages: &[Any],
        fee: Fee,
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
            Fee::Manual(fee) => fee,
            Fee::Auto(multiplier) => auto_fee(multiplier).await?,
            Fee::PayerGranterAuto(multiplier, payer, granter) => {
                let mut fee = auto_fee(multiplier).await?;
                fee.payer = payer;
                fee.granter = granter;
                fee
            }
        };
        debug!("Fee used for the transaction: {:?}", fee);
        Ok(fee)
    }

    /// Broadcast a transaction, returning immediately.
    async fn sign_and_broadcast_async(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_async::Response, NymdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;
        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_async(self, tx_bytes.into()).await
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    async fn sign_and_broadcast_sync(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_sync::Response, NymdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;
        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_sync(self, tx_bytes.into()).await
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    async fn sign_and_broadcast_commit(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;

        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_commit(self, tx_bytes.into()).await
    }

    /// Broadcast a transaction to the network and monitors its inclusion in a block.
    async fn sign_and_broadcast(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NymdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;

        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        self.broadcast_tx(tx_bytes.into()).await
    }

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, NymdError> {
        let signer_accounts = self.signer().try_derive_accounts()?;
        let account_from_signer = signer_accounts
            .iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| NymdError::SigningAccountNotFound(signer_address.clone()))?;

        // TODO: WTF HOW IS TIMEOUT_HEIGHT SUPPOSED TO GET DETERMINED?
        // IT DOESNT EXIST IN COSMJS!!
        // try to set to 0
        let timeout_height = 0u32;

        let tx_body = tx::Body::new(messages, memo, timeout_height);
        let signer_info =
            SignerInfo::single_direct(Some(account_from_signer.public_key), signer_data.sequence);
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

        self.signer()
            .sign_direct_with_account(account_from_signer, sign_doc)
    }

    async fn sign(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<tx::Raw, NymdError> {
        // TODO: Future optimisation: rather than grabbing current account_number and sequence
        // on every sign request -> just keep them cached on the struct and increment as required
        let sequence_response = self.get_sequence(signer_address).await?;
        let chain_id = self.get_chain_id().await?;

        let signer_data = SignerData {
            account_number: sequence_response.account_number,
            sequence: sequence_response.sequence,
            chain_id,
        };

        self.sign_direct(signer_address, messages, fee, memo, signer_data)
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    rpc_client: HttpClient,
    signer: DirectSecp256k1HdWallet,
    gas_price: GasPrice,

    broadcast_polling_rate: Duration,
    broadcast_timeout: Duration,
}

impl Client {
    pub fn connect_with_signer<U: Clone>(
        endpoint: U,
        signer: DirectSecp256k1HdWallet,
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

#[async_trait]
impl SigningCosmWasmClient for Client {
    fn signer(&self) -> &DirectSecp256k1HdWallet {
        &self.signer
    }

    fn gas_price(&self) -> &GasPrice {
        &self.gas_price
    }
}
