// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_ecash_contract_common::events::TICKET_BOOK_VALUE;
use nym_ecash_contract_common::msg::ExecuteMsg as EcashExecuteMsg;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait EcashSigningClient {
    async fn execute_ecash_contract(
        &self,
        fee: Option<Fee>,
        msg: EcashExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn make_ticketbook_deposit(
        &self,
        public_key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = EcashExecuteMsg::DepositTicketBookFunds {
            identity_key: public_key,
        };
        let amount = Coin::new(TICKET_BOOK_VALUE, "unym");
        self.execute_ecash_contract(fee, req, "Ecash::Deposit".to_string(), vec![amount])
            .await
    }

    async fn request_ticket_redemption(
        &self,
        commitment_bs58: String,
        number_of_tickets: u16,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = EcashExecuteMsg::RequestRedemption {
            commitment_bs58,
            number_of_tickets,
        };
        self.execute_ecash_contract(fee, req, Default::default(), vec![])
            .await
    }

    async fn propose_for_blacklist(
        &self,
        public_key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = EcashExecuteMsg::ProposeToBlacklist { public_key };
        self.execute_ecash_contract(fee, req, "Ecash::ProposeToBlacklist".to_string(), vec![])
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> EcashSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_ecash_contract(
        &self,
        fee: Option<Fee>,
        msg: EcashExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let ecash_contract_address = self
            .ecash_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("coconut bandwidth contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let signer_address = &self.signer_addresses()?[0];

        self.execute(
            signer_address,
            ecash_contract_address,
            &msg,
            fee,
            memo,
            funds,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;
    use nym_ecash_contract_common::msg::ExecuteMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: EcashSigningClient + Send + Sync>(
        client: C,
        msg: EcashExecuteMsg,
    ) {
        match msg {
            EcashExecuteMsg::DepositTicketBookFunds { identity_key } => client
                .make_ticketbook_deposit(identity_key.to_string(), None)
                .ignore(),
            EcashExecuteMsg::AddToBlacklist { public_key: _ } => unimplemented!(), //no add to blacklist method on client
            EcashExecuteMsg::ProposeToBlacklist { public_key } => {
                client.propose_for_blacklist(public_key, None).ignore()
            }
            ExecuteMsg::RequestRedemption {
                commitment_bs58,
                number_of_tickets,
            } => client
                .request_ticket_redemption(commitment_bs58, number_of_tickets, None)
                .ignore(),
            ExecuteMsg::RedeemTickets { .. } => unimplemented!(), // no redeem tickets method for the client
        };
    }
}
