#![forbid(unsafe_code)]

pub mod authentication;
pub(crate) mod client;
pub(crate) mod mixnet_responses;
mod request;
pub mod server;
pub mod types;
pub mod utils;

/// Version of socks
const SOCKS_VERSION: u8 = 0x05;

const RESERVED: u8 = 0x00;
