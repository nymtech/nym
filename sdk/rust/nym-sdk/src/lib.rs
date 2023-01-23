//! Rust SDK for the Nym platform
//!
//! The main component currently is [`mixnet`].

mod error;

pub mod mixnet;

pub use error::{Error, Result};
