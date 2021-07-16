// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use crate::ValidatorClientError;
use async_trait::async_trait;
use cosmos_sdk::bank::MsgSend;
use cosmos_sdk::proto::cosmwasm::wasm::v1beta1::MsgExecuteContract;
use cosmos_sdk::rpc::endpoint::broadcast;
use cosmos_sdk::rpc::{
    Client, Error as TendermintRpcError, HttpClient, HttpClientUrl, SimpleRequest,
};
use cosmos_sdk::tendermint::{block, chain};
use cosmos_sdk::tx::{AccountNumber, Fee, Msg, MsgType, SequenceNumber, SignDoc, SignerInfo};
use cosmos_sdk::{cosmwasm, rpc, tx, AccountId, Coin};
use serde::Serialize;
use std::convert::TryInto;
use std::pin::Pin;

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

// TODO: implement those
type UploadMeta = ();
type UploadResult = ();
type InstantiateOptions = ();
type InstantiateResult = ();
type ChangeAdminResult = ();
type MigrateResult = ();
type ExecuteResult = ();

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

    pub async fn upload(
        &self,
        sender_address: &AccountId,
        wasm_code: Vec<u8>,
        fee: Fee,
        meta: Option<UploadMeta>,
        memo: impl Into<String>,
    ) -> Result<UploadResult, ValidatorClientError> {
        let upload_msg = cosmwasm::MsgStoreCode {
            sender: sender_address.clone(),
            wasm_byte_code: wasm_code,
            source: todo!(),
            builder: todo!(),
            instantiate_permission: todo!(),
        }
        .to_msg()
        .map_err(|_| ValidatorClientError::SerializationError("MsgStoreCode".to_owned()))?;

        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![upload_msg], fee, memo)
            .await?;

        todo!("produce change upload result here")
    }

    pub async fn instantiate<M>(
        &self,
        sender_address: &AccountId,
        code_id: u64,
        msg: &M,
        label: String,
        fee: Fee,
        options: Option<InstantiateOptions>,
        memo: impl Into<String>,
    ) -> Result<InstantiateResult, ValidatorClientError>
    where
        M: ?Sized + Serialize,
    {
        let init_msg = cosmwasm::MsgInstantiateContract {
            sender: sender_address.clone(),
            admin: todo!(),
            code_id,
            label: todo!(),
            init_msg: serde_json::to_vec(msg)?,
            funds: todo!(),
        }
        .to_msg()
        .map_err(|_| {
            ValidatorClientError::SerializationError("MsgInstantiateContract".to_owned())
        })?;

        // TODO: cosmjs doesn't have a memo here
        let tx_res = self
            .sign_and_broadcast_commit(sender_address, vec![init_msg], fee, memo)
            .await?;

        todo!("produce init result here")
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
            .await?;

        todo!("produce change admin result here")
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
            .await?;

        todo!("produce change admin result here")
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
            .await?;

        todo!("produce migrate result here")
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
            .await?;

        todo!("produce execute result here")
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
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        todo!()
    }

    pub async fn undelegate_tokens(
        delegator_address: &AccountId,
        validator_address: &AccountId,
        amount: Coin,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        todo!()
    }

    pub async fn withdraw_rewards(
        delegator_address: &AccountId,
        validator_address: &AccountId,
        fee: Fee,
        memo: impl Into<String>,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        todo!()
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
        let (account_number, sequence) = self.base_client.get_sequence(signer_address).await?;
        let chain_id = self.base_client.get_chain_id().await?;

        let signer_data = SignerData {
            account_number,
            sequence,
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
