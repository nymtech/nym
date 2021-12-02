// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::helpers::{compress_wasm_code, CheckResponse};
use crate::nymd::cosmwasm_client::logs::{self, parse_raw_logs};
use crate::nymd::cosmwasm_client::types::*;
use crate::nymd::error::NymdError;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use async_trait::async_trait;
use cosmrs::bank::MsgSend;
use cosmrs::distribution::MsgWithdrawDelegatorReward;
use cosmrs::rpc::endpoint::broadcast;
use cosmrs::rpc::{Error as TendermintRpcError, HttpClient, HttpClientUrl, SimpleRequest};
use cosmrs::staking::{MsgDelegate, MsgUndelegate};
use cosmrs::tx::{Fee, Msg, SignDoc, SignerInfo};
use cosmrs::{cosmwasm, rpc, tx, AccountId, Any, Coin};
use log::debug;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::convert::TryInto;

#[async_trait]
pub trait SigningCosmWasmClient: CosmWasmClient {
    fn signer(&self) -> &DirectSecp256k1HdWallet;

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
            .sign_and_broadcast_commit(sender_address, vec![upload_msg], fee, memo)
            .await?
            .check_response()?;

        let logs = parse_raw_logs(tx_res.deliver_tx.log)?;

        // TODO: should those strings be extracted into some constants?
        // the reason I think unwrap here is fine is that if the transaction succeeded and those
        // fields do not exist or code_id is not a number, there's no way we can recover, we're probably connected
        // to wrong validator or something
        let code_id = logs::find_attribute(&logs, "message", "code_id")
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
            admin: options
                .as_mut()
                .map(|options| options.admin.take())
                .flatten(),
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
            .sign_and_broadcast_commit(sender_address, vec![init_msg], fee, memo)
            .await?
            .check_response()?;

        let logs = parse_raw_logs(tx_res.deliver_tx.log)?;

        // TODO: should those strings be extracted into some constants?
        // the reason I think unwrap here is fine is that if the transaction succeeded and those
        // fields do not exist or address is malformed, there's no way we can recover, we're probably connected
        // to wrong validator or something
        let contract_address = logs::find_attribute(&logs, "message", "contract_address")
            .unwrap()
            .value
            .parse()
            .unwrap();

        Ok(InstantiateResult {
            contract_address,
            logs,
            transaction_hash: tx_res.hash,
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
            .sign_and_broadcast_commit(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(ChangeAdminResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
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
            .sign_and_broadcast_commit(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(ChangeAdminResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
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
            .sign_and_broadcast_commit(sender_address, vec![migrate_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(MigrateResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
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
            funds,
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgExecuteContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![execute_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
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
                    funds,
                }
                .to_any()
                .map_err(|_| NymdError::SerializationError("MsgExecuteContract".to_owned()))
            })
            .collect::<Result<_, _>>()?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, messages, fee, memo)
            .await?
            .check_response()?;

        debug!(
            "gas wanted: {:?}, gas used: {:?}",
            tx_res.deliver_tx.gas_wanted, tx_res.deliver_tx.gas_used
        );

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
        })
    }

    async fn send_tokens(
        &self,
        sender_address: &AccountId,
        recipient_address: &AccountId,
        amount: Vec<Coin>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError> {
        let send_msg = MsgSend {
            from_address: sender_address.clone(),
            to_address: recipient_address.clone(),
            amount,
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgSend".to_owned()))?;

        self.sign_and_broadcast_commit(sender_address, vec![send_msg], fee, memo)
            .await
    }

    async fn delegate_tokens(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError> {
        let delegate_msg = MsgDelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount,
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgDelegate".to_owned()))?;

        self.sign_and_broadcast_commit(delegator_address, vec![delegate_msg], fee, memo)
            .await
    }

    async fn undelegate_tokens(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError> {
        let undelegate_msg = MsgUndelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: Some(amount),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgUndelegate".to_owned()))?;

        self.sign_and_broadcast_commit(delegator_address, vec![undelegate_msg], fee, memo)
            .await
    }

    async fn withdraw_rewards(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError> {
        let withdraw_msg = MsgWithdrawDelegatorReward {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
        }
        .to_any()
        .map_err(|_| NymdError::SerializationError("MsgWithdrawDelegatorReward".to_owned()))?;

        self.sign_and_broadcast_commit(delegator_address, vec![withdraw_msg], fee, memo)
            .await
    }

    /// Broadcast a transaction, returning immediately.
    async fn sign_and_broadcast_async(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_async::Response, NymdError> {
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
        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NymdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_commit(self, tx_bytes.into()).await
    }

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
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
        fee: Fee,
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

#[derive(Debug)]
pub struct Client {
    rpc_client: HttpClient,
    signer: DirectSecp256k1HdWallet,
}

impl Client {
    pub fn connect_with_signer<U>(
        endpoint: U,
        signer: DirectSecp256k1HdWallet,
    ) -> Result<Self, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let rpc_client = HttpClient::new(endpoint)?;
        Ok(Client { rpc_client, signer })
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
impl CosmWasmClient for Client {}

#[async_trait]
impl SigningCosmWasmClient for Client {
    fn signer(&self) -> &DirectSecp256k1HdWallet {
        &self.signer
    }
}
