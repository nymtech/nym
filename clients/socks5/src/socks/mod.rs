#![forbid(unsafe_code)]

pub mod authentication;
mod client;
mod request;
pub mod server;
pub mod types;
pub mod utils;

/// Version of socks
const SOCKS_VERSION: u8 = 0x05;

const RESERVED: u8 = 0x00;
