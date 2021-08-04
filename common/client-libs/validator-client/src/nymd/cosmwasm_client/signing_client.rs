// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::types::*;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use crate::ValidatorClientError;
// use async_trait::async_trait;
use crate::nymd::cosmwasm_client::helpers::CheckResponse;
use crate::nymd::cosmwasm_client::logs::{self, parse_raw_logs};
use cosmos_sdk::bank::MsgSend;
use cosmos_sdk::distribution::MsgWithdrawDelegatorReward;
use cosmos_sdk::rpc::endpoint::broadcast;
use cosmos_sdk::rpc::{Error as TendermintRpcError, HttpClientUrl};
use cosmos_sdk::staking::{MsgDelegate, MsgUndelegate};
use cosmos_sdk::tendermint::chain;
use cosmos_sdk::tx::{AccountNumber, Fee, Msg, MsgType, SequenceNumber, SignDoc, SignerInfo};
use cosmos_sdk::{cosmwasm, tx, AccountId, Coin};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::convert::TryInto;
use std::io::Write;

// TODO: move it elsewhere
struct SignerData {
    account_number: AccountNumber,
    sequence: SequenceNumber,
    chain_id: chain::Id,
}

pub struct SigningCosmWasmClient {
    base_client: CosmWasmClient,
    signer: DirectSecp256k1HdWallet,
}

impl SigningCosmWasmClient {
    pub fn connect_with_signer<U>(
        endpoint: U,
        signer: DirectSecp256k1HdWallet,
    ) -> Result<Self, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(SigningCosmWasmClient {
            base_client: CosmWasmClient::connect(endpoint)?,
            signer,
        })
    }

    fn compress_wasm_code(&self, code: &[u8]) -> Result<Vec<u8>, ValidatorClientError> {
        // using compression level 9, same as cosmjs, that optimises for size
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(code)
            .map_err(ValidatorClientError::WasmCompressionError)?;
        encoder
            .finish()
            .map_err(ValidatorClientError::WasmCompressionError)
    }

    pub async fn upload(
        &self,
        sender_address: &AccountId,
        wasm_code: Vec<u8>,
        fee: Fee,
        memo: impl Into<String>,
        mut meta: Option<UploadMeta>,
    ) -> Result<UploadResult, ValidatorClientError> {
        let compressed = self.compress_wasm_code(&wasm_code)?;
        let compressed_size = compressed.len();
        let compressed_checksum = Sha256::digest(&compressed).to_vec();

        // TODO: what about instantiate_permission?
        // cosmjs is just ignoring that field...
        let upload_msg = cosmwasm::MsgStoreCode {
            sender: sender_address.clone(),
            wasm_byte_code: compressed,
            source: meta
                .as_mut()
                .map(|meta| meta.source.take())
                .unwrap_or_default(),
            builder: meta
                .as_mut()
                .map(|meta| meta.builder.take())
                .unwrap_or_default(),
            instantiate_permission: Default::default(),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgStoreCode".to_owned()))?;

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
    pub async fn instantiate<M>(
        &self,
        sender_address: &AccountId,
        code_id: ContractCodeId,
        msg: &M,
        label: String,
        fee: Fee,
        memo: impl Into<String>,
        mut options: Option<InstantiateOptions>,
    ) -> Result<InstantiateResult, ValidatorClientError>
    where
        M: ?Sized + Serialize,
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
            init_msg: serde_json::to_vec(msg)?,
            funds: options.map(|options| options.funds).unwrap_or_default(),
        }
        .to_msg()
        .map_err(|_| {
            ValidatorClientError::SerializationError("MsgInstantiateContract".to_owned())
        })?;

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

    pub async fn update_admin(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        new_admin: &AccountId,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<ChangeAdminResult, ValidatorClientError> {
        let change_admin_msg = cosmwasm::MsgUpdateAdmin {
            sender: sender_address.clone(),
            new_admin: new_admin.clone(),
            contract: contract_address.clone(),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgUpdateAdmin".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(ChangeAdminResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
        })
    }

    pub async fn clear_admin(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<ChangeAdminResult, ValidatorClientError> {
        let change_admin_msg = cosmwasm::MsgClearAdmin {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgClearAdmin".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![change_admin_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(ChangeAdminResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
        })
    }

    pub async fn migrate<M>(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        code_id: u64,
        fee: Fee,
        msg: &M,
        memo: impl Into<String>,
    ) -> Result<MigrateResult, ValidatorClientError>
    where
        M: ?Sized + Serialize,
    {
        let migrate_msg = cosmwasm::MsgMigrateContract {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
            code_id,
            migrate_msg: serde_json::to_vec(msg)?,
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgMigrateContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![migrate_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(MigrateResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
        })
    }

    pub async fn execute<M>(
        &self,
        sender_address: &AccountId,
        contract_address: &AccountId,
        msg: &M,
        fee: Fee,
        memo: impl Into<String>,
        funds: Option<Vec<Coin>>,
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        M: ?Sized + Serialize,
    {
        let execute_msg = cosmwasm::MsgExecuteContract {
            sender: sender_address.clone(),
            contract: contract_address.clone(),
            msg: serde_json::to_vec(msg)?,
            funds: funds.unwrap_or_default(),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgExecuteContract".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![execute_msg], fee, memo)
            .await?
            .check_response()?;

        Ok(ExecuteResult {
            logs: parse_raw_logs(tx_res.deliver_tx.log)?,
            transaction_hash: tx_res.hash,
        })
    }

    pub async fn send_tokens(
        &self,
        sender_address: &AccountId,
        recipient_address: &AccountId,
        amount: Vec<Coin>,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        let send_msg = MsgSend {
            from_address: sender_address.clone(),
            to_address: recipient_address.clone(),
            amount,
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgSend".to_owned()))?;

        self.sign_and_broadcast_commit(sender_address, vec![send_msg], fee, memo)
            .await
    }

    pub async fn delegate_tokens(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        let delegate_msg = MsgDelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: Some(amount),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgDelegate".to_owned()))?;

        self.sign_and_broadcast_commit(delegator_address, vec![delegate_msg], fee, memo)
            .await
    }

    pub async fn undelegate_tokens(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        let undelegate_msg = MsgUndelegate {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
            amount: Some(amount),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgUndelegate".to_owned()))?;

        self.sign_and_broadcast_commit(delegator_address, vec![undelegate_msg], fee, memo)
            .await
    }

    pub async fn withdraw_rewards(
        &self,
        delegator_address: &AccountId,
        validator_address: &AccountId,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        let withdraw_msg = MsgWithdrawDelegatorReward {
            delegator_address: delegator_address.to_owned(),
            validator_address: validator_address.to_owned(),
        }
        .to_msg()
        .map_err(|_| {
            ValidatorClientError::SerializationError("MsgWithdrawDelegatorReward".to_owned())
        })?;

        self.sign_and_broadcast_commit(delegator_address, vec![withdraw_msg], fee, memo)
            .await
    }

    // Creates a transaction with the given messages, fee and memo. Then signs and broadcasts the transaction.

    /// Broadcast a transaction, returning immediately.
    pub async fn sign_and_broadcast_async(
        &self,
        signer_address: &AccountId,
        messages: Vec<Msg>,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_async::Response, ValidatorClientError> {
        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| ValidatorClientError::SerializationError("Tx".to_owned()))?;

        self.base_client.broadcast_tx_async(tx_bytes.into()).await
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    pub async fn sign_and_broadcast_sync(
        &self,
        signer_address: &AccountId,
        messages: Vec<Msg>,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_sync::Response, ValidatorClientError> {
        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| ValidatorClientError::SerializationError("Tx".to_owned()))?;

        self.base_client.broadcast_tx_sync(tx_bytes.into()).await
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    pub async fn sign_and_broadcast_commit(
        &self,
        signer_address: &AccountId,
        messages: Vec<Msg>,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        let tx_raw = self.sign(signer_address, messages, fee, memo).await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|_| ValidatorClientError::SerializationError("Tx".to_owned()))?;

        self.base_client.broadcast_tx_commit(tx_bytes.into()).await
    }

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Msg>,
        fee: Fee,
        memo: impl Into<String>,
        signer_data: SignerData,
    ) -> Result<tx::Raw, ValidatorClientError> {
        let signer_accounts = self.signer.get_accounts();
        let account_from_signer = signer_accounts
            .iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| ValidatorClientError::SigningAccountNotFound(signer_address.clone()))?;

        // TODO: WTF HOW IS TIMEOUT_HEIGHT SUPPOSED TO GET DETERMINED?
        // IT DOESNT EXIST IN COSMJS!!
        // try to set to 0
        let timeout_height = 0u32;

        let tx_body = tx::Body::new(messages, memo, timeout_height);
        let signer_info =
            SignerInfo::single_direct(Some(account_from_signer.public_key), signer_data.sequence);
        let auth_info = signer_info.auth_info(fee);

        // ideally I'd prefer to have the entire error put into the ValidatorClientError::SigningFailure
        // but I'm super hesitant to trying to downcast the eyre::Report to cosmos_sdk::error::Error
        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &signer_data.chain_id,
            signer_data.account_number,
        )
        .map_err(|_| ValidatorClientError::SigningFailure)?;

        self.signer.sign_direct(signer_address, sign_doc)
    }

    pub async fn sign(
        &self,
        signer_address: &AccountId,
        messages: Vec<Msg>,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<tx::Raw, ValidatorClientError> {
        // TODO: Rather than grabbing current account_number and sequence
        // on every sign request -> just keep them cached on the struct and increment as required
        let sequence_response = self.base_client.get_sequence(signer_address).await?;
        let chain_id = self.base_client.get_chain_id().await?;

        let signer_data = SignerData {
            account_number: sequence_response.account_number,
            sequence: sequence_response.sequence,
            chain_id,
        };

        self.sign_direct(signer_address, messages, fee, memo, signer_data)
    }

    // TODO: here be the ugliness of re-exposing methods from base_client
}

// #[async_trait]
// pub trait SigningCosmWasmClient: QueryCosmWasmClient {
//     async fn foo(&self);
//     // /**
//     //  * Creates a client in offline mode.
//     //  *
//     //  * This should only be used in niche cases where you know exactly what you're doing,
//     //  * e.g. when building an offline signing application.
//     //  *
//     //  * When you try to use online functionality with such a signer, an
//     //  * exception will be raised.
//     //  */
//     // static offline(signer: OfflineSigner, options?: SigningCosmWasmClientOptions): Promise<SigningCosmWasmClient>;
//     // protected constructor(tmClient: Tendermint34Client | undefined, signer: OfflineSigner, options: SigningCosmWasmClientOptions);
//     // /** Uploads code and returns a receipt, including the code ID */
//     // upload(sender_address: string, wasmCode: Uint8Array, meta?: UploadMeta, memo: impl Into<String>): Promise<UploadResult>;
//     // instantiate(senderAddress: string, code_id: number, msg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult>;
//     // update_admin(senderAddress: string, contract_address: string, newAdmin: string, memo: impl Into<String>): Promise<ChangeAdminResult>;
//     // clear_admin(senderAddress: string, contract_address: string, memo: impl Into<String>): Promise<ChangeAdminResult>;
//     // migrate(senderAddress: string, contractAddress: string, code_id: number, migrateMsg: Record<string, unknown>, memo: impl Into<String>): Promise<MigrateResult>;
//     // execute(senderAddress: string, contractAddress: string, msg: Record<string, unknown>, memo: impl Into<String>, funds?: readonly Coin[]): Promise<ExecuteResult>;
//     // send_tokens(senderAddress: string, recipient_address: string, amount: readonly Coin[], memo: impl Into<String>): Promise<broadcast::tx_commit::Response>;
//     // delegate_tokens(delegator_address: string, validatorAddress: string, amount: Coin, memo: impl Into<String>): Promise<broadcast::tx_commit::Response>;
//     // undelegate_tokens(delegator_address: string, validatorAddress: string, amount: Coin, memo: impl Into<String>): Promise<broadcast::tx_commit::Response>;
//     // withdraw_rewards(delegator_address: string, validatorAddress: string, memo: impl Into<String>): Promise<broadcast::tx_commit::Response>;
//     // /**
//     //  * Creates a transaction with the given messages, fee and memo. Then signs and broadcasts the transaction.
//     //  *
//     //  * @param signer_address The address that will sign transactions using this instance. The signer must be able to sign with this address.
//     //  * @param messages
//     //  * @param fee
//     //  * @param memo
//     //  */
//     // sign_and_broadcast(signer_address: string, messages: readonly EncodeObject[], fee: StdFee, memo: impl Into<String>): Promise<broadcast::tx_commit::Response>;
//     // sign(signer_address: string, messages: readonly EncodeObject[], fee: StdFee, memo: string, explicitSignerData?: SignerData): Promise<TxRaw>;
// }

// impl Client {
//     async fn foo(&self) {
//         let bar = self.http_client.status().await.unwrap();
//         println!("{}", bar.sync_info.latest_block_height.value())
//     }
// }

// #[async_trait]
// impl SigningCosmWasmClient for Client {
//     async fn foo(&self) {
//
//     }
// }
//
// #[async_trait]
// impl QueryCosmWasmClient for Client {}
//
// #[async_trait]
// impl rpc::Client for Client {
//     async fn perform<R>(&self, request: R) -> rpc::Result<R::Response>
//     where
//         R: SimpleRequest,
//     {
//         self.http_client.perform(request).await
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::QueryRequest;
    use mixnet_contract::PagedMixnodeResponse;

    #[tokio::test]
    async fn foo() {
        // let mut client = SigningCosmWasmClient::connect_with_signer()
        // let validator = "http://127.0.0.1:26657";
        // let contract = "hal10pyejy66429refv3g35g2t7am0was7yam2dd72"
        //     .parse::<AccountId>()
        //     .unwrap();

        let validator = "https://testnet-milhon-validator1.nymtech.net";
        let contract = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen"
            .parse::<AccountId>()
            .unwrap();
        let mnemonic = "bitter east decide match spare blue shadow trouble share dice surface cave hospital poem whip message army behind elephant mom horse leg purity below";

        let wallet = DirectSecp256k1HdWallet::builder()
            .with_prefix("hal")
            .build(mnemonic.parse().unwrap())
            .unwrap();

        let address = wallet.get_accounts()[0].address.clone();

        let client = SigningCosmWasmClient::connect_with_signer(validator, wallet).unwrap();

        // // let balance = client.base_client.get_all_balances(&address).await.unwrap();
        // let res = client.base_client.get_account(&address).await.unwrap();
        //
        // println!("{:?}", res);

        let aaa = QueryRequest::GetMixNodes {
            limit: None,
            start_after: None,
        };

        let mixes = client
            .base_client
            .query_contract_smart::<_, PagedMixnodeResponse>(&contract, &aaa)
            .await
            .unwrap();

        println!("mixes: {:?}", mixes);
    }
}
