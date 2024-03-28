// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod message;
pub mod node;
pub mod processor;
pub mod receiver;
pub mod tester;

pub use message::{Empty, TestMessage};
pub use nym_sphinx::{
    chunking::fragment::FragmentIdentifier, params::PacketSize, preparer::PreparedFragment,
};
pub use tester::NodeTester;

// it feels wrong to redefine it, but I don't want to import the whole of contract commons just for this one type
pub(crate) type NodeId = u32;

#[macro_export]
macro_rules! log_err {
    ($($t:tt)*) => {{
        #[cfg(target_arch = "wasm32")]
        {::wasm_utils::console_error!($($t)*)}

        #[cfg(not(target_arch = "wasm32"))]
        {::log::error!($($t)*)}
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($($t:tt)*) => {{
        #[cfg(target_arch = "wasm32")]
        {::wasm_utils::console_warn!($($t)*)}

        #[cfg(not(target_arch = "wasm32"))]
        {::log::warn!($($t)*)}
    }};
}

#[macro_export]
macro_rules! log_info {
    ($($t:tt)*) => {{
        #[cfg(target_arch = "wasm32")]
        {::wasm_utils::console_log!($($t)*)}

        #[cfg(not(target_arch = "wasm32"))]
        {::log::info!($($t)*)}
    }};
}
