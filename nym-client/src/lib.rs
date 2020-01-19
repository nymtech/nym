#![recursion_limit = "256"]

pub mod clients;
pub mod persistence;
pub mod sockets;
pub mod utils;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));    
}