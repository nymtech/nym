// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nymsphinx_acknowledgements::surb_ack::SurbAckRecoveryError;
use nymsphinx_addressing::nodes::NymNodeRoutingAddressError;
use nymsphinx_types::Error as SphinxError;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum MixProcessingError {
    SphinxProcessingError(SphinxError),
    InvalidHopAddress(NymNodeRoutingAddressError),
    NoSurbAckInFinalHop,
    MalformedSurbAck(SurbAckRecoveryError),
}

impl From<SphinxError> for MixProcessingError {
    // for the time being just have a single error instance for all possible results of SphinxError
    fn from(err: SphinxError) -> Self {
        use MixProcessingError::*;

        SphinxProcessingError(err)
    }
}

impl From<NymNodeRoutingAddressError> for MixProcessingError {
    fn from(err: NymNodeRoutingAddressError) -> Self {
        use MixProcessingError::*;

        InvalidHopAddress(err)
    }
}

impl From<SurbAckRecoveryError> for MixProcessingError {
    fn from(err: SurbAckRecoveryError) -> Self {
        use MixProcessingError::*;

        MalformedSurbAck(err)
    }
}

impl Display for MixProcessingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MixProcessingError::SphinxProcessingError(sphinx_err) => {
                write!(f, "Sphinx Processing Error - {}", sphinx_err)
            }
            MixProcessingError::InvalidHopAddress(address_err) => {
                write!(f, "Invalid Hop Address - {:?}", address_err)
            }
            MixProcessingError::NoSurbAckInFinalHop => {
                write!(f, "No SURBAck present in the final hop data")
            }
            MixProcessingError::MalformedSurbAck(surb_ack_err) => {
                write!(f, "Malformed SURBAck - {:?}", surb_ack_err)
            }
        }
    }
}

impl std::error::Error for MixProcessingError {}
