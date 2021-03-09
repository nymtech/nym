pub mod contract;
pub mod error;
pub mod msg;
pub mod queries;
pub mod state;
pub mod support;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
