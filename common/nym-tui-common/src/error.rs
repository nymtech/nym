// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::char::ParseCharError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymTuiError {
    #[error("failed to abort tui processing task within specified duration")]
    TuiTaskAbortFailure,

    #[error("{str} could not be parsed into a character: {source}")]
    InvalidCharacter {
        str: String,
        #[source]
        source: ParseCharError,
    },

    #[error("could not process an unknown keybinding: '{value}'")]
    UnknownKeyBinding { value: String },

    #[error("could not process an unknown key modifier: '{value}'")]
    UnknownKeyModifier { value: String },
}
