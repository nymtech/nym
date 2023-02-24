// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod contract_mock;
mod error;
mod execution;
mod helpers;
mod mock_api;
mod multi_contract_mock;
mod raw_state;
mod single_contract_mock;
mod traits;

pub use contract_mock::ContractState;
pub use error::MockingError;
pub use helpers::{deserialize_msg, env_with_block_info, serialize_msg};

#[cfg(feature = "state-importing")]
pub use raw_state::ImportedContractState;

#[cfg(feature = "contract-mocks")]
pub use multi_contract_mock::MultiContractMock;

#[cfg(feature = "contract-mocks")]
pub use single_contract_mock::SingleContractMock;

#[cfg(feature = "contract-mocks")]
pub use execution::{
    CrossContractTokenMove, ExecutionResult, ExecutionStepResult, FurtherExecution,
};

#[cfg(feature = "testable-trait")]
pub use traits::TestableContract;

pub const AVERAGE_BLOCKTIME_SECS: u64 = 5;
