use addressing::AddressTypeError;
use log::*;
use rand::{thread_rng, Rng};
use sphinx::route::{Destination, Node};
use std::collections::HashMap;
use std::convert::TryInto;
use std::net::SocketAddr;
use std::time;
use topology::{NymTopology, NymTopologyError};

pub mod fragment;
pub mod set;
#[derive(PartialEq, Debug)]
pub enum ChunkingError {
    TooBigMessageToSplit,
    MalformedHeaderError,
    NoValidProvidersError,
    NoValidRoutesAvailableError,
    InvalidTopologyError,
    TooShortFragmentData,
    MalformedFragmentData,
    UnexpectedFragmentCount,
}

impl From<topology::NymTopologyError> for ChunkingError {
    fn from(_: NymTopologyError) -> Self {
        use ChunkingError::*;
        NoValidRoutesAvailableError
    }
}

// this will later be completely removed as `addressing` is moved into this crate
impl From<AddressTypeError> for ChunkingError {
    fn from(_: AddressTypeError) -> Self {
        use ChunkingError::*;
        InvalidTopologyError
    }
}
