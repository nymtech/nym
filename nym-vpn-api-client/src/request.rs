// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountRequestBody {
    pub account_addr: String,
    pub pub_key: String,
    pub signature_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequestBody {
    pub device_identity_key: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestZkNymRequestBody {
    pub withdrawal_request: String,
    pub ecash_pubkey: String,
    pub expiration_date: String,
    pub ticketbook_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyFreepassRequestBody {
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSubscriptionInvoicesRequestBody {
    pub subscription: String,
    pub date: String,
    pub status: CreateSubscriptionInvoicesStatus,
    pub invoice_no: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateSubscriptionInvoicesStatus {
    Unpaid,
    Paid,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSubscriptionRequestBody {
    pub valid_from_utc: String,
    pub subscription_kind: CreateSubscriptionKind,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateSubscriptionKind {
    OneMonth,
    OneYear,
    TwoYears,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestRefundRequestBody {
    subscription_invoice: String,
    status: RequestRefundRequestStatus,
    user_reason: RequestRefundRequestUserReason,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestRefundRequestStatus {
    Pending,
    Complete,
    Rejected,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestRefundRequestUserReason {
    SubscriptionInError,
    PoorPerformance,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeviceRequestBody {
    pub status: UpdateDeviceRequestStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateDeviceRequestStatus {
    Active,
    Inactive,
    DeleteMe,
}
