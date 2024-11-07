// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_credentials_interface::{PublicKeyUser, TicketType, WithdrawalRequest};
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use time::{Date, OffsetDateTime};

#[cfg(feature = "query-types")]
use nym_http_api_common::Output;

#[cfg(feature = "tsify")]
use tsify::Tsify;
use uuid::Uuid;

#[cfg(feature = "tsify")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PlaceholderJsonSchemaImpl {}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TicketbookRequest {
    /// base58 encoded withdrawal request
    pub withdrawal_request: WithdrawalRequestBs58Wrapper,

    /// bs58-encoded **ECASH** public key.
    /// this is **NOT** a device key or anything like that.
    /// it is derived from user's **SECRET** key!
    ///
    /// you **MUST** provide a valid value otherwise blacklisting won't work
    #[schemars(with = "String")]
    #[serde(with = "bs58_ecash")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub ecash_pubkey: PublicKeyUser,

    // needs to be explicit in case user creates request at 23:59:59.999, but it reaches vpn-api at 00:00:00.001
    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    pub expiration_date: Date,

    #[schemars(with = "String")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub ticketbook_type: TicketType,

    pub is_freepass_request: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TicketbookAsyncRequest {
    #[serde(flatten)]
    pub inner: TicketbookRequest,

    /// unique id of the device
    pub device_id: String,
    /// unique id of the credential
    pub credential_id: String,
    /// secret used for webhook responses
    pub secret: String,
}

mod bs58_ecash {
    use nym_credentials_interface::Base58;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer, T>(req: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Base58,
    {
        serializer.serialize_str(&req.to_bs58())
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T>(deserializer: D) -> Result<T, D::Error>
    where
        T: Base58,
    {
        let s = <String>::deserialize(deserializer)?;
        T::try_from_bs58(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String))]
pub struct WithdrawalRequestBs58Wrapper(#[serde(with = "bs58_ecash")] pub WithdrawalRequest);

impl From<WithdrawalRequestBs58Wrapper> for WithdrawalRequest {
    fn from(value: WithdrawalRequestBs58Wrapper) -> Self {
        value.0
    }
}

impl From<WithdrawalRequest> for WithdrawalRequestBs58Wrapper {
    fn from(value: WithdrawalRequest) -> Self {
        WithdrawalRequestBs58Wrapper(value)
    }
}

impl Deref for WithdrawalRequestBs58Wrapper {
    type Target = WithdrawalRequest;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WithdrawalRequestBs58Wrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// implementation taken from: https://github.com/GREsau/schemars/pull/207
impl JsonSchema for WithdrawalRequestBs58Wrapper {
    fn is_referenceable() -> bool {
        true
    }

    fn schema_name() -> String {
        "WithdrawalRequestBs58Wrapper".into()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        // during serialisation we just use bs58 representation
        String::json_schema(gen)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct CurrentEpochResponse {
    pub epoch_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct PartialVerificationKeysResponse {
    pub epoch_id: u64,
    pub keys: Vec<PartialVerificationKey>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct PartialVerificationKey {
    pub node_index: u64,
    pub bs58_encoded_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct MasterVerificationKeyResponse {
    pub epoch_id: u64,
    pub bs58_encoded_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DepositResponse {
    pub current_deposit_amount: u128,
    pub current_deposit_denom: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct AggregatedExpirationDateSignaturesResponse {
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    #[cfg_attr(feature = "openapi", schema(value_type = PlaceholderJsonSchemaImpl))]
    pub signatures: AggregatedExpirationDateSignatures,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct AggregatedCoinIndicesSignaturesResponse {
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    #[cfg_attr(feature = "openapi", schema(value_type = PlaceholderJsonSchemaImpl))]
    pub signatures: AggregatedCoinIndicesSignatures,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct WalletShare {
    pub node_index: u64,
    pub bs58_encoded_share: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TicketbookWalletSharesResponse {
    pub epoch_id: u64,
    pub shares: Vec<WalletShare>,
    pub master_verification_key: Option<MasterVerificationKeyResponse>,
    pub aggregated_coin_index_signatures: Option<AggregatedCoinIndicesSignaturesResponse>,
    pub aggregated_expiration_date_signatures: Option<AggregatedExpirationDateSignaturesResponse>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TicketbookWalletSharesAsyncResponse {
    pub id: i64,

    // maybe redundant, but could be useful for debugging
    pub uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct WebhookTicketbookWalletShares {
    pub id: i64,
    pub status: String,
    pub device_id: String,
    pub credential_id: String,
    pub data: Option<TicketbookWalletSharesResponse>,
    pub error_message: Option<String>,

    #[schemars(with = "String")]
    #[serde(with = "time::serde::rfc3339")]
    pub created: OffsetDateTime,

    #[schemars(with = "String")]
    #[serde(with = "time::serde::rfc3339")]
    pub updated: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct WebhookTicketbookWalletSharesRequest {
    pub ticketbook_wallet_shares: WebhookTicketbookWalletShares,
    pub secret: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg(feature = "query-types")]
#[serde(default, rename_all = "kebab-case")]
pub struct TicketbookObtainQueryParams {
    pub output: Option<Output>,

    pub include_master_verification_key: bool,

    pub include_coin_index_signatures: bool,

    pub include_expiration_date_signatures: bool,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg(feature = "query-types")]
#[serde(default, rename_all = "kebab-case")]
pub struct SharesQueryParams {
    pub output: Option<Output>,

    pub include_master_verification_key: bool,

    pub include_coin_index_signatures: bool,

    pub include_expiration_date_signatures: bool,
}
