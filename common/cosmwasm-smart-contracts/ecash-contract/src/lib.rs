// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared types, messages, events, and errors for the ecash contract.
//!
//! Consumed by both the contract crate (`contracts/ecash`) and any off-chain
//! client (gateways, nym-api signers, indexers, validator-client). See
//! `openspec/specs/ecash-contract/spec.md` for the normative interface.

pub mod blacklist;
pub mod counters;
pub mod deposit;
pub mod deposit_statistics;
pub mod error;
pub mod event_attributes;
pub mod events;
pub mod msg;
pub mod redeem_credential;
pub mod reduced_deposit;

pub use error::EcashContractError;
