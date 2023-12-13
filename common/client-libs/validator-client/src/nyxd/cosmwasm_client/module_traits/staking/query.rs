// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, PageRequest};
use async_trait::async_trait;
use cosmrs::proto::cosmos::staking::v1beta1::{
    QueryHistoricalInfoRequest as ProtoQueryHistoricalInfoRequest,
    QueryHistoricalInfoResponse as ProtoQueryHistoricalInfoResponse,
    QueryValidatorRequest as ProtoQueryValidatorRequest,
    QueryValidatorResponse as ProtoQueryValidatorResponse,
    QueryValidatorsRequest as ProtoQueryValidatorsRequest,
    QueryValidatorsResponse as ProtoQueryValidatorsResponse,
};
use cosmrs::staking::{
    QueryHistoricalInfoRequest, QueryHistoricalInfoResponse, QueryValidatorRequest,
    QueryValidatorResponse, QueryValidatorsRequest, QueryValidatorsResponse,
};
use cosmrs::AccountId;

// TODO: change trait restriction from `CosmWasmClient` to `TendermintRpcClient`
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait StakingQueryClient: CosmWasmClient {
    async fn historical_info(&self, height: i64) -> Result<QueryHistoricalInfoResponse, NyxdError> {
        let path = Some("/cosmos.staking.v1beta1.Query/HistoricalInfo".to_owned());

        let req = QueryHistoricalInfoRequest { height };

        let res = self
            .make_abci_query::<ProtoQueryHistoricalInfoRequest, ProtoQueryHistoricalInfoResponse>(
                path,
                req.into(),
            )
            .await?;

        Ok(res.try_into()?)
    }

    async fn validator(
        &self,
        validator_addr: AccountId,
    ) -> Result<QueryValidatorResponse, NyxdError> {
        let path = Some("/cosmos.staking.v1beta1.Query/Validator".to_owned());

        let req = QueryValidatorRequest { validator_addr };

        let res = self
            .make_abci_query::<ProtoQueryValidatorRequest, ProtoQueryValidatorResponse>(
                path,
                req.into(),
            )
            .await?;

        Ok(res.try_into()?)
    }

    async fn validators(
        &self,
        status: String,
        pagination: Option<PageRequest>,
    ) -> Result<QueryValidatorsResponse, NyxdError> {
        let path = Some("/cosmos.staking.v1beta1.Query/Validators".to_owned());

        let req = QueryValidatorsRequest { status, pagination };

        let res = self
            .make_abci_query::<ProtoQueryValidatorsRequest, ProtoQueryValidatorsResponse>(
                path,
                req.into(),
            )
            .await?;

        Ok(res.try_into()?)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> StakingQueryClient for T where T: CosmWasmClient {}
