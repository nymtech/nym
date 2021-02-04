pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod types;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
