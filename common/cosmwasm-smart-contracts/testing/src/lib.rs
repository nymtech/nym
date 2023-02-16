// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod contract_mock;
mod error;
mod execution;
mod multi_contract_mock;
mod raw_state;

pub use contract_mock::{env_with_block_info, ContractMock};
pub use error::MockingError;
pub use execution::{
    CrossContractTokenMove, ExecutionResult, ExecutionStepResult, FurtherExecution,
};
pub use multi_contract_mock::{DuplicateContractAddress, MultiContractMock, TestableContract};
pub use raw_state::ContractState;

pub const AVERAGE_BLOCKTIME_SECS: u64 = 5;

// pub(crate) type InstantiateHandler<I, E> =
//     fn(&DepsMut<'_>, Env, MessageInfo, I) -> Result<Response, E>;
// pub(crate) type ExecuteHandler<EX, E> =
//     fn(&DepsMut<'_>, Env, MessageInfo, EX) -> Result<Response, E>;
// pub(crate) type QueryHandler<Q, E> = fn(&Deps<'_>, Env, Q) -> Result<QueryResponse, E>;
// pub(crate) type MigrateHandler<M, E> = fn(&DepsMut<'_>, Env, MessageInfo, M) -> Result<Response, E>;
