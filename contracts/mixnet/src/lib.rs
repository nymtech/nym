pub mod contract;
pub mod error;
pub mod msg;
pub mod queries;
pub mod state;
pub mod support;
pub mod transactions;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points_with_migration!(contract);
