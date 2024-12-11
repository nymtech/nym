// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::http::state::ChainClient;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::nyxd::cosmwasm_client::ContractResponseData;
use nym_validator_client::nyxd::{Coin, Hash};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub(crate) struct DepositResponse {
    pub tx_hash: Hash,
    pub deposit_id: DepositId,
}

pub(crate) struct DepositRequest {
    pubkey: ed25519::PublicKey,
    deposit_amount: Coin,
    on_done: oneshot::Sender<Option<DepositResponse>>,
}

impl DepositRequest {
    pub(crate) fn new(
        pubkey: ed25519::PublicKey,
        deposit_amount: &Coin,
        on_done: oneshot::Sender<Option<DepositResponse>>,
    ) -> Self {
        DepositRequest {
            pubkey,
            deposit_amount: deposit_amount.clone(),
            on_done,
        }
    }
}

pub(crate) type DepositRequestReceiver = mpsc::Receiver<DepositRequest>;

pub(crate) fn new_control_channels(
    max_concurrent_deposits: usize,
) -> (DepositRequestSender, DepositRequestReceiver) {
    let (tx, rx) = mpsc::channel(max_concurrent_deposits);
    (tx.into(), rx)
}

#[derive(Debug, Clone)]
pub struct DepositRequestSender(mpsc::Sender<DepositRequest>);

impl From<mpsc::Sender<DepositRequest>> for DepositRequestSender {
    fn from(inner: mpsc::Sender<DepositRequest>) -> Self {
        DepositRequestSender(inner)
    }
}

impl DepositRequestSender {
    pub(crate) async fn request_deposit(&self, request: DepositRequest) {
        if self.0.send(request).await.is_err() {
            error!("failed to request deposit: the DepositMaker must have died!")
        }
    }
}

pub(crate) struct DepositMaker {
    client: ChainClient,
    max_concurrent_deposits: usize,
    deposit_request_sender: DepositRequestSender,
    deposit_request_receiver: DepositRequestReceiver,
    short_sha: &'static str,
    cancellation_token: CancellationToken,
}

impl DepositMaker {
    pub(crate) fn new(
        short_sha: &'static str,
        client: ChainClient,
        max_concurrent_deposits: usize,
        cancellation_token: CancellationToken,
    ) -> Self {
        let (deposit_request_sender, deposit_request_receiver) =
            new_control_channels(max_concurrent_deposits);

        DepositMaker {
            client,
            max_concurrent_deposits,
            deposit_request_sender,
            deposit_request_receiver,
            short_sha,
            cancellation_token,
        }
    }

    pub(crate) fn deposit_request_sender(&self) -> DepositRequestSender {
        self.deposit_request_sender.clone()
    }

    pub(crate) async fn process_deposit_requests(
        &mut self,
        requests: Vec<DepositRequest>,
    ) -> Result<(), VpnApiError> {
        let chain_write_permit = self.client.start_chain_tx().await;

        info!("starting deposits");
        let mut contents = Vec::new();
        let mut replies = Vec::new();
        for request in requests {
            // check if the channel is still open in case the receiver client has cancelled the request
            if request.on_done.is_closed() {
                warn!(
                    "the request for deposit from {} got cancelled",
                    request.pubkey
                );
                continue;
            }

            contents.push((request.pubkey.to_base58_string(), request.deposit_amount));
            replies.push(request.on_done);
        }

        let deposits_res = chain_write_permit
            .make_deposits(self.short_sha, contents)
            .await;
        let execute_res = match deposits_res {
            Ok(res) => res,
            Err(err) => {
                // we have to let requesters know the deposit(s) failed
                for reply in replies {
                    if reply.send(None).is_err() {
                        warn!("one of the deposit requesters has been terminated")
                    }
                }
                return Err(err);
            }
        };

        let tx_hash = execute_res.transaction_hash;
        info!("{} deposits made in transaction: {tx_hash}", replies.len());

        let contract_data = match execute_res.to_contract_data() {
            Ok(contract_data) => contract_data,
            Err(err) => {
                // that one is tricky. deposits technically got made, but we somehow failed to parse response,
                // in this case terminate the proxy with 0 exit code so it wouldn't get automatically restarted
                // because it requires some serious MANUAL intervention
                error!("CRITICAL FAILURE: failed to parse out deposit information from the contract transaction. either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually. error was: {err}");
                self.cancellation_token.cancel();
                return Err(VpnApiError::DepositFailure);
            }
        };

        if contract_data.len() != replies.len() {
            // another critical failure, that one should be quite impossible and thus has to be manually inspected
            error!("CRITICAL FAILURE: failed to parse out all deposit information from the contract transaction. got {} responses while we sent {} deposits! either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually", contract_data.len(), replies.len());
            self.cancellation_token.cancel();
            return Err(VpnApiError::DepositFailure);
        }

        for (reply_channel, response) in replies.into_iter().zip(contract_data) {
            let response_index = response.message_index;
            let deposit_id = match response.parse_singleton_u32_contract_data() {
                Ok(deposit_id) => deposit_id,
                Err(err) => {
                    // another impossibility
                    error!("CRITICAL FAILURE: failed to parse out deposit id out of the response at index {response_index}: {err}. either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually");
                    self.cancellation_token.cancel();
                    return Err(VpnApiError::DepositFailure);
                }
            };

            if reply_channel
                .send(Some(DepositResponse {
                    deposit_id,
                    tx_hash,
                }))
                .is_err()
            {
                warn!("one of the deposit requesters has been terminated. deposit {deposit_id} will remain unclaimed!");
                // this shouldn't happen as the requester task shouldn't be killed, but it's not a critical failure
                // we just lost some tokens, but it's not an undefined on-chain behaviour
            }
        }

        Ok(())
    }

    pub async fn run_forever(mut self) {
        info!("starting the deposit maker task");
        loop {
            let mut receive_buffer = Vec::with_capacity(self.max_concurrent_deposits);
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    break
                }
                received = self.deposit_request_receiver.recv_many(&mut receive_buffer, self.max_concurrent_deposits) => {
                    debug!("received {received} deposit requests");
                    if let Err(err) = self.process_deposit_requests(receive_buffer).await {
                        error!("failed to process received deposit requests: {err}")
                    }
                }
            }
        }
    }
}
