// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::client_traits::CosmWasmClient;
use crate::nyxd::cosmwasm_client::helpers::{
    compress_wasm_code, parse_msg_responses, CheckResponse,
};
use crate::nyxd::cosmwasm_client::logs::{self, parse_raw_logs};
use crate::nyxd::cosmwasm_client::types::*;
use crate::nyxd::error::NyxdError;
use crate::nyxd::fee::{Fee, DEFAULT_SIMULATED_GAS_MULTIPLIER};
use crate::nyxd::{Coin, GasAdjustable, GasPrice, TxResponse};
use crate::signing::signer::OfflineSigner;
use crate::signing::tx_signer::TxSigner;
use crate::signing::SignerData;
use async_trait::async_trait;
use cosmrs::bank::MsgSend;
use cosmrs::cosmwasm::{MsgClearAdmin, MsgUpdateAdmin};
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
use std::time::SystemTime;
use tendermint_rpc::endpoint::broadcast;

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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait SigningCosmWasmClient: CosmWasmClient + TxSigner
where
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    // TODO: would it somehow be possible to get rid of this method and allow for
    // blanket implementation for anything that provides CosmWasmClient + TxSigner?
    fn gas_price(&self) -> &GasPrice;

    fn simulated_gas_multiplier(&self) -> f32;

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
        let change_admin_msg = MsgUpdateAdmin {
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
        let change_admin_msg = MsgClearAdmin {
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

        println!("execute");

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.tx_result.log)?,
            msg_responses: parse_msg_responses(tx_res.tx_result.data),
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
            msg_responses: parse_msg_responses(tx_res.tx_result.data),
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
                .map_err(|_| NyxdError::SerializationError("MsgSend".to_owned()))
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

            debug!("Gas estimation: {gas_estimation}");
            debug!("Multiplying the estimation by {multiplier}");
            debug!("Final gas limit used: {gas}");

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

        self.broadcast_tx(tx_bytes, None, None).await
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

        Ok(<Self as TxSigner>::sign_direct(
            self,
            signer_address,
            messages,
            fee,
            memo,
            signer_data,
        )?)
    }
}
