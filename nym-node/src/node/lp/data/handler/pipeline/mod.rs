// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Data pipelines. The wire wrapping/unwrapping
//! shared between them lives in [`wire`].

// Mixing node only
mod mixing_node;

// Full blown node with client handling capacity
mod nymnode;
pub mod wire;

pub(crate) use mixing_node::MixingNodeDataPipeline;
pub use nymnode::NymNodeDataPipeline;
pub(crate) use wire::LpTransport;
