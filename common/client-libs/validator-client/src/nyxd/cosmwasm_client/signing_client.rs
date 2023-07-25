// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::client::CosmWasmClient;
use crate::nyxd::cosmwasm_client::helpers::{compress_wasm_code, CheckResponse};
use crate::nyxd::cosmwasm_client::logs::{self, parse_raw_logs};
use crate::nyxd::cosmwasm_client::types::*;
use crate::nyxd::error::NyxdError;
use crate::nyxd::fee::{Fee, DEFAULT_SIMULATED_GAS_MULTIPLIER};
use crate::nyxd::{Coin, GasAdjustable, GasPrice, TxResponse};
use crate::signing::signer::OfflineSigner;
use crate::signing::SignerData;
use async_trait::async_trait;
use cosmrs::abci::GasInfo;
use cosmrs::bank::MsgSend;
use cosmrs::distribution::MsgWithdrawDelegatorReward;
use cosmrs::feegrant::{
    AllowedMsgAllowance, BasicAllowance, MsgGrantAllowance, MsgRevokeAllowance,
};
use cosmrs::proto::cosmos::tx::signing::v1beta1::SignMode;
use cosmrs::staking::{MsgDelegate, MsgUndelegate};
use cosmrs::tx::{self, Msg};
use cosmrs::{cosmwasm, AccountId, Any, Tx};
use log::debug;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::convert::TryInto;
use std::time::{Duration, SystemTime};
use tendermint_rpc::endpoint::broadcast;

#[cfg(feature = "http-client")]
use crate::signing::tx_signer::TxSigner;

#[cfg(feature = "http-client")]
use tendermint_rpc::{Error as TendermintRpcError, SimpleRequest};

#[cfg(feature = "http-client")]
use cosmrs::rpc::{HttpClient, HttpClientUrl};

pub const DEFAULT_BROADCAST_POLLING_RATE: Duration = Duration::from_secs(4);
pub const DEFAULT_BROADCAST_TIMEOUT: Duration = Duration::from_secs(60);

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
    type Signer: OfflineSigner + Send + Sync;

    fn signer(&self) -> &Self::Signer;

    fn gas_price(&self) -> &GasPrice;

    fn signer_public_key(&self, signer_address: &AccountId) -> Option<tx::SignerPublicKey> {
        let account = self.signer().find_account(signer_address).ok()?;
        Some(account.public_key().into())
    }

    async fn simulate(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<SimulateResponse, NyxdError> {
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
    ) -> Result<UploadResult, NyxdError> {
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
        .map_err(|_| NyxdError::SerializationError("MsgStoreCode".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![upload_msg], fee, memo)
            .await?
            .check_response()?;

        let logs = parse_raw_logs(tx_res.tx_result.log)?;
        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };

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
    ) -> Result<InstantiateResult, NyxdError>
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
        .map_err(|_| NyxdError::SerializationError("MsgInstantiateContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![init_msg], fee, memo)
            .await?
            .check_response()?;

        let logs = parse_raw_logs(tx_res.tx_result.log)?;
        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };
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
    ) -> Result<ChangeAdminResult, NyxdError> {
        let change_admin_msg = sealed::cosmwasm::MsgUpdateAdmin {
            sender: sender_address.clone(),
            new_admin: new_admin.clone(),
            contract: contract_address.clone(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgUpdateAdmin".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };
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
    ) -> Result<ChangeAdminResult, NyxdError> {
        let change_admin_msg = sealed::cosmwasm::MsgClearAdmin {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgClearAdmin".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };
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
    ) -> Result<MigrateResult, NyxdError>
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
        .map_err(|_| NyxdError::SerializationError("MsgMigrateContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![migrate_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };
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
    ) -> Result<ExecuteResult, NyxdError>
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
        .map_err(|_| NyxdError::SerializationError("MsgExecuteContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast(sender_address, vec![execute_msg], fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };
        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            data: tx_res.tx_result.data.into(),
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
    ) -> Result<ExecuteResult, NyxdError>
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
                .map_err(|_| NyxdError::SerializationError("MsgExecuteContract".to_owned()))
            })
            .collect::<Result<_, _>>()?;

        let tx_res = self
            .sign_and_broadcast(sender_address, messages, fee, memo)
            .await?
            .check_response()?;

        let gas_info = GasInfo {
            gas_wanted: tx_res.tx_result.gas_wanted.try_into().unwrap_or_default(),
            gas_used: tx_res.tx_result.gas_used.try_into().unwrap_or_default(),
        };
        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            data: tx_res.tx_result.data.into(),
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
    ) -> Result<TxResponse, NyxdError> {
        let send_msg = MsgSend {
            from_address: sender_address.clone(),
            to_address: recipient_address.clone(),
            amount: amount.into_iter().map(Into::into).collect(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgSend".to_owned()))?;

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
    ) -> Result<TxResponse, NyxdError>
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
                .map_err(|_| NyxdError::SerializationError("MsgExecuteContract".to_owned()))
            })
            .collect::<Result<_, _>>()?;

        self.sign_and_broadcast(sender_address, messages, fee, memo)
            .await?
            .check_response()
    }

    #[allow(clippy::too_many_arguments)]
    async fn grant_allowance(
        &self,
        granter: &AccountId,
        grantee: &AccountId,
        spend_limit: Vec<Coin>,
        expiration: Option<SystemTime>,
        allowed_messages: Vec<String>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NyxdError> {
        let basic_allowance = BasicAllowance {
            spend_limit: spend_limit.into_iter().map(Into::into).collect(),
            expiration,
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("BasicAllowance".to_owned()))?;

        let allowed_msg_allowance = AllowedMsgAllowance {
            allowance: Some(basic_allowance),
            allowed_messages,
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("AllowedMsgAllowance".to_owned()))?;

        let grant_allowance_msg = MsgGrantAllowance {
            granter: granter.to_owned(),
            grantee: grantee.to_owned(),
            allowance: Some(allowed_msg_allowance),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgGrantAllowance".to_owned()))?;

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
    ) -> Result<TxResponse, NyxdError> {
        let revoke_allowance_msg = MsgRevokeAllowance {
            granter: granter.to_owned(),
            grantee: grantee.to_owned(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgRevokeAllowance".to_owned()))?;

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
    ) -> Result<TxResponse, NyxdError> {
        let delegate_msg = MsgDelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: amount.into(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgDelegate".to_owned()))?;

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
    ) -> Result<TxResponse, NyxdError> {
        let undelegate_msg = MsgUndelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: amount.into(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgUndelegate".to_owned()))?;

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
    ) -> Result<TxResponse, NyxdError> {
        let withdraw_msg = MsgWithdrawDelegatorReward {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
        }
        .to_any()
        .map_err(|_| NyxdError::SerializationError("MsgWithdrawDelegatorReward".to_owned()))?;

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
    ) -> Result<tx::Fee, NyxdError> {
        let auto_fee = |multiplier: Option<f32>| async move {
            debug!("Trying to simulate gas costs...");
            // from what I've seen in manual testing, gas estimation does not exist if transaction
            // fails to get executed (for example if you send 'BondMixnode" with invalid signature)
            let gas_estimation = self
                .simulate(signer_address, messages.to_vec(), memo.clone())
                .await?
                .gas_info
                .ok_or(NyxdError::GasEstimationFailure)?
                .gas_used;

            let multiplier = multiplier.unwrap_or(DEFAULT_SIMULATED_GAS_MULTIPLIER);
            let gas = gas_estimation.adjust_gas(multiplier);

            debug!("Gas estimation: {}", gas_estimation);
            debug!("Multiplying the estimation by {}", multiplier);
            debug!("Final gas limit used: {}", gas);

            let fee = self.gas_price() * gas;
            Ok::<tx::Fee, NyxdError>(tx::Fee::from_amount_and_gas(fee, gas))
        };
        let fee = match fee {
            Fee::Manual(fee) => fee,
            Fee::Auto(multiplier) => auto_fee(multiplier).await?,
            Fee::PayerGranterAuto(auto_feegrant) => {
                let mut fee = auto_fee(auto_feegrant.gas_adjustment).await?;
                fee.payer = auto_feegrant.payer;
                fee.granter = auto_feegrant.granter;
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
    ) -> Result<broadcast::tx_async::Response, NyxdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;
        let tx_raw = self.sign(signer_address, messages, fee, memo, None).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NyxdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_async(self, tx_bytes).await
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    async fn sign_and_broadcast_sync(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_sync::Response, NyxdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;
        let tx_raw = self.sign(signer_address, messages, fee, memo, None).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NyxdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_sync(self, tx_bytes).await
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    async fn sign_and_broadcast_commit(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NyxdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;

        let tx_raw = self.sign(signer_address, messages, fee, memo, None).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NyxdError::SerializationError("Tx".to_owned()))?;

        CosmWasmClient::broadcast_tx_commit(self, tx_bytes).await
    }

    /// Broadcast a transaction to the network and monitors its inclusion in a block.
    async fn sign_and_broadcast(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<TxResponse, NyxdError> {
        let memo = memo.into();
        let fee = self
            .determine_transaction_fee(signer_address, &messages, fee, &memo)
            .await?;

        let tx_raw = self.sign(signer_address, messages, fee, memo, None).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| NyxdError::SerializationError("Tx".to_owned()))?;

        self.broadcast_tx(tx_bytes).await
    }

    async fn sign(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        explicit_signer_data: Option<SignerData>,
    ) -> Result<tx::Raw, NyxdError> {
        let signer_data = match explicit_signer_data {
            Some(signer_data) => signer_data,
            None => {
                // TODO: Future optimisation: rather than grabbing current account_number and sequence
                // on every sign request -> just keep them cached on the struct and increment as required
                let sequence_response = self.get_sequence(signer_address).await?;
                let chain_id = self.get_chain_id().await?;

                SignerData::new_from_sequence_response(sequence_response, chain_id)
            }
        };

        self.sign_direct(signer_address, messages, fee, memo, signer_data)
    }

    fn sign_amino(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, NyxdError>;

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, NyxdError>;
}

#[cfg(feature = "http-client")]
#[derive(Debug)]
pub struct Client<S> {
    // TODO: somehow nicely hide this guy if we decide to use our client in offline mode,
    // maybe just convert it into an option?
    // or maybe we need another level of indirection. tbd.
    rpc_client: HttpClient,
    tx_signer: TxSigner<S>,
    gas_price: GasPrice,

    broadcast_polling_rate: Duration,
    broadcast_timeout: Duration,
}

#[cfg(feature = "http-client")]
impl<S> Client<S> {
    pub fn connect_with_signer<U: Clone>(
        endpoint: U,
        signer: S,
        gas_price: GasPrice,
    ) -> Result<Self, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let rpc_client = HttpClient::new(endpoint)?;
        Ok(Client {
            rpc_client,
            tx_signer: TxSigner::new(signer),
            gas_price,
            broadcast_polling_rate: DEFAULT_BROADCAST_POLLING_RATE,
            broadcast_timeout: DEFAULT_BROADCAST_TIMEOUT,
        })
    }

    pub fn offline(signer: S) -> TxSigner<S>
    where
        S: OfflineSigner,
    {
        TxSigner::new(signer)
    }

    pub fn change_endpoint<U>(&mut self, new_endpoint: U) -> Result<(), NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let new_rpc_client = HttpClient::new(new_endpoint)?;
        self.rpc_client = new_rpc_client;
        Ok(())
    }

    pub fn into_signer(self) -> S {
        self.tx_signer.into_inner_signer()
    }

    pub fn set_broadcast_polling_rate(&mut self, broadcast_polling_rate: Duration) {
        self.broadcast_polling_rate = broadcast_polling_rate
    }

    pub fn set_broadcast_timeout(&mut self, broadcast_timeout: Duration) {
        self.broadcast_timeout = broadcast_timeout
    }
}

#[cfg(feature = "http-client")]
#[async_trait]
impl<S> tendermint_rpc::client::Client for Client<S>
where
    S: Send + Sync,
{
    async fn perform<R>(&self, request: R) -> Result<R::Output, tendermint_rpc::Error>
    where
        R: SimpleRequest,
    {
        self.rpc_client.perform(request).await
    }
}

#[cfg(feature = "http-client")]
#[async_trait]
impl<S> CosmWasmClient for Client<S>
where
    S: Send + Sync,
{
    fn broadcast_polling_rate(&self) -> Duration {
        self.broadcast_polling_rate
    }

    fn broadcast_timeout(&self) -> Duration {
        self.broadcast_timeout
    }
}

#[cfg(feature = "http-client")]
#[async_trait]
impl<S> SigningCosmWasmClient for Client<S>
where
    S: OfflineSigner + Send + Sync,
    NyxdError: From<S::Error>,
{
    type Signer = S;

    fn signer(&self) -> &Self::Signer {
        self.tx_signer.signer()
    }

    fn gas_price(&self) -> &GasPrice {
        &self.gas_price
    }

    fn sign_amino(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, NyxdError> {
        Ok(self
            .tx_signer
            .sign_amino(signer_address, messages, fee, memo, signer_data)?)
    }

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, NyxdError> {
        Ok(self
            .tx_signer
            .sign_direct(signer_address, messages, fee, memo, signer_data)?)
    }
}

// a temporary bypass until https://github.com/cosmos/cosmos-rust/pull/419 is merged
mod sealed {
    pub mod cosmwasm {
        use cosmrs::{proto, tx::Msg, AccountId, ErrorReport, Result};

        /// MsgUpdateAdmin sets a new admin for a smart contract
        #[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
        pub struct MsgUpdateAdmin {
            /// Sender is the that actor that signed the messages
            pub sender: AccountId,

            /// NewAdmin address to be set
            pub new_admin: AccountId,

            /// Contract is the address of the smart contract
            pub contract: AccountId,
        }

        impl Msg for MsgUpdateAdmin {
            type Proto = proto::cosmwasm::wasm::v1::MsgUpdateAdmin;
        }

        impl TryFrom<proto::cosmwasm::wasm::v1::MsgUpdateAdmin> for MsgUpdateAdmin {
            type Error = ErrorReport;

            fn try_from(
                proto: proto::cosmwasm::wasm::v1::MsgUpdateAdmin,
            ) -> Result<MsgUpdateAdmin> {
                MsgUpdateAdmin::try_from(&proto)
            }
        }

        impl TryFrom<&proto::cosmwasm::wasm::v1::MsgUpdateAdmin> for MsgUpdateAdmin {
            type Error = ErrorReport;

            fn try_from(
                proto: &proto::cosmwasm::wasm::v1::MsgUpdateAdmin,
            ) -> Result<MsgUpdateAdmin> {
                Ok(MsgUpdateAdmin {
                    sender: proto.sender.parse()?,
                    new_admin: proto.new_admin.parse()?,
                    contract: proto.contract.parse()?,
                })
            }
        }

        impl From<MsgUpdateAdmin> for proto::cosmwasm::wasm::v1::MsgUpdateAdmin {
            fn from(msg: MsgUpdateAdmin) -> proto::cosmwasm::wasm::v1::MsgUpdateAdmin {
                proto::cosmwasm::wasm::v1::MsgUpdateAdmin::from(&msg)
            }
        }

        impl From<&MsgUpdateAdmin> for proto::cosmwasm::wasm::v1::MsgUpdateAdmin {
            fn from(msg: &MsgUpdateAdmin) -> proto::cosmwasm::wasm::v1::MsgUpdateAdmin {
                proto::cosmwasm::wasm::v1::MsgUpdateAdmin {
                    sender: msg.sender.to_string(),
                    new_admin: msg.new_admin.to_string(),
                    contract: msg.contract.to_string(),
                }
            }
        }

        /// MsgUpdateAdminResponse returns empty data
        #[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
        pub struct MsgUpdateAdminResponse {}

        impl Msg for MsgUpdateAdminResponse {
            type Proto = proto::cosmwasm::wasm::v1::MsgUpdateAdminResponse;
        }

        impl TryFrom<proto::cosmwasm::wasm::v1::MsgUpdateAdminResponse> for MsgUpdateAdminResponse {
            type Error = ErrorReport;

            fn try_from(
                _proto: proto::cosmwasm::wasm::v1::MsgUpdateAdminResponse,
            ) -> Result<MsgUpdateAdminResponse> {
                Ok(MsgUpdateAdminResponse {})
            }
        }

        impl From<MsgUpdateAdminResponse> for proto::cosmwasm::wasm::v1::MsgUpdateAdminResponse {
            fn from(
                _msg: MsgUpdateAdminResponse,
            ) -> proto::cosmwasm::wasm::v1::MsgUpdateAdminResponse {
                proto::cosmwasm::wasm::v1::MsgUpdateAdminResponse {}
            }
        }

        /// MsgClearAdmin removes any admin stored for a smart contract
        #[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
        pub struct MsgClearAdmin {
            /// Sender is the that actor that signed the messages
            pub sender: AccountId,

            /// Contract is the address of the smart contract
            pub contract: AccountId,
        }

        impl Msg for MsgClearAdmin {
            type Proto = proto::cosmwasm::wasm::v1::MsgClearAdmin;
        }

        impl TryFrom<proto::cosmwasm::wasm::v1::MsgClearAdmin> for MsgClearAdmin {
            type Error = ErrorReport;

            fn try_from(proto: proto::cosmwasm::wasm::v1::MsgClearAdmin) -> Result<MsgClearAdmin> {
                MsgClearAdmin::try_from(&proto)
            }
        }

        impl TryFrom<&proto::cosmwasm::wasm::v1::MsgClearAdmin> for MsgClearAdmin {
            type Error = ErrorReport;

            fn try_from(proto: &proto::cosmwasm::wasm::v1::MsgClearAdmin) -> Result<MsgClearAdmin> {
                Ok(MsgClearAdmin {
                    sender: proto.sender.parse()?,
                    contract: proto.contract.parse()?,
                })
            }
        }

        impl From<MsgClearAdmin> for proto::cosmwasm::wasm::v1::MsgClearAdmin {
            fn from(msg: MsgClearAdmin) -> proto::cosmwasm::wasm::v1::MsgClearAdmin {
                proto::cosmwasm::wasm::v1::MsgClearAdmin::from(&msg)
            }
        }

        impl From<&MsgClearAdmin> for proto::cosmwasm::wasm::v1::MsgClearAdmin {
            fn from(msg: &MsgClearAdmin) -> proto::cosmwasm::wasm::v1::MsgClearAdmin {
                proto::cosmwasm::wasm::v1::MsgClearAdmin {
                    sender: msg.sender.to_string(),
                    contract: msg.contract.to_string(),
                }
            }
        }

        /// MsgClearAdminResponse returns empty data
        #[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
        pub struct MsgClearAdminResponse {}

        impl Msg for MsgClearAdminResponse {
            type Proto = proto::cosmwasm::wasm::v1::MsgClearAdminResponse;
        }

        impl TryFrom<proto::cosmwasm::wasm::v1::MsgClearAdminResponse> for MsgClearAdminResponse {
            type Error = ErrorReport;

            fn try_from(
                _proto: proto::cosmwasm::wasm::v1::MsgClearAdminResponse,
            ) -> Result<MsgClearAdminResponse> {
                Ok(MsgClearAdminResponse {})
            }
        }

        impl From<MsgClearAdminResponse> for proto::cosmwasm::wasm::v1::MsgClearAdminResponse {
            fn from(
                _msg: MsgClearAdminResponse,
            ) -> proto::cosmwasm::wasm::v1::MsgClearAdminResponse {
                proto::cosmwasm::wasm::v1::MsgClearAdminResponse {}
            }
        }
    }
}
