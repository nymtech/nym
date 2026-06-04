// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// pub use cosmrs::feegrant::{Q}

// unfortunately queries are not implemented within cosmrs

use crate::nyxd::error::NyxdError;
use crate::nyxd::{Any, CosmWasmClient};
use async_trait::async_trait;
use cosmrs::AccountId;

use cosmrs::proto::cosmos::feegrant::v1beta1::{
    QueryAllowancesRequest as ProtoQueryAllowancesRequest,
    QueryAllowancesResponse as ProtoQueryAllowancesResponse,
};

#[derive(Clone, Debug, PartialEq)]
pub struct QueryAllowancesRequest {
    pub grantee: AccountId,

    pub pagination: Option<cosmrs::query::PageRequest>,
}

impl TryFrom<cosmrs::proto::cosmos::feegrant::v1beta1::QueryAllowancesRequest>
    for QueryAllowancesRequest
{
    type Error = cosmrs::ErrorReport;

    fn try_from(
        value: cosmrs::proto::cosmos::feegrant::v1beta1::QueryAllowancesRequest,
    ) -> Result<Self, Self::Error> {
        Ok(QueryAllowancesRequest {
            grantee: value.grantee.parse()?,
            pagination: value.pagination.map(Into::into),
        })
    }
}

impl From<QueryAllowancesRequest>
    for cosmrs::proto::cosmos::feegrant::v1beta1::QueryAllowancesRequest
{
    fn from(value: QueryAllowancesRequest) -> Self {
        Self {
            grantee: value.grantee.to_string(),
            pagination: value.pagination.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryAllowancesResponse {
    pub allowances: Vec<Grant>,
    pub pagination: Option<cosmrs::query::PageResponse>,
}

impl TryFrom<cosmrs::proto::cosmos::feegrant::v1beta1::QueryAllowancesResponse>
    for QueryAllowancesResponse
{
    type Error = cosmrs::ErrorReport;

    fn try_from(
        value: cosmrs::proto::cosmos::feegrant::v1beta1::QueryAllowancesResponse,
    ) -> Result<Self, Self::Error> {
        Ok(QueryAllowancesResponse {
            allowances: value
                .allowances
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            pagination: value.pagination.map(Into::into),
        })
    }
}

impl From<QueryAllowancesResponse>
    for cosmrs::proto::cosmos::feegrant::v1beta1::QueryAllowancesResponse
{
    fn from(value: QueryAllowancesResponse) -> Self {
        Self {
            allowances: value.allowances.into_iter().map(Into::into).collect(),
            pagination: value.pagination.map(Into::into),
        }
    }
}

/// Grant is stored in the KVStore to record a grant with full context
#[derive(Clone, Debug, PartialEq)]
pub struct Grant {
    /// granter is the address of the user granting an allowance of their funds.
    pub granter: AccountId,

    /// grantee is the address of the user being granted an allowance of another user's funds.
    pub grantee: AccountId,

    /// allowance can be any of basic, periodic, allowed fee allowance.
    pub allowance: Option<Any>,
}

impl TryFrom<cosmrs::proto::cosmos::feegrant::v1beta1::Grant> for Grant {
    type Error = cosmrs::ErrorReport;

    fn try_from(
        value: cosmrs::proto::cosmos::feegrant::v1beta1::Grant,
    ) -> Result<Self, Self::Error> {
        Ok(Grant {
            granter: value.granter.parse()?,
            grantee: value.grantee.parse()?,
            allowance: value.allowance,
        })
    }
}

impl From<Grant> for cosmrs::proto::cosmos::feegrant::v1beta1::Grant {
    fn from(value: Grant) -> Self {
        Self {
            granter: value.granter.to_string(),
            grantee: value.grantee.to_string(),
            allowance: value.allowance,
        }
    }
}

// TODO: change trait restriction from `CosmWasmClient` to `TendermintRpcClient`
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait FeegrantQueryClient: CosmWasmClient {
    async fn allowances(
        &self,
        grantee: AccountId,
        pagination: Option<cosmrs::query::PageRequest>,
    ) -> Result<QueryAllowancesResponse, NyxdError> {
        let path = Some("/cosmos.feegrant.v1beta1.Query/Allowances".to_owned());

        let req = QueryAllowancesRequest {
            grantee,
            pagination,
        };

        let res = self
            .make_abci_query::<ProtoQueryAllowancesRequest, ProtoQueryAllowancesResponse>(
                path,
                req.into(),
            )
            .await?;

        Ok(res.try_into()?)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> FeegrantQueryClient for T where T: CosmWasmClient {}
