// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ffi::OsString;
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum NymvisorError {
    #[error(
    "failed to load config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the provided env file was malformed: {source}")]
    MalformedEnvFile {
        #[from]
        source: dotenvy::Error,
    },

    #[error("the value provided for environmental variable '{variable}' was not valid unicode: {value:?}")]
    MalformedEnvVariable { variable: String, value: OsString },

    #[error("the value provided for environmental boolean variable '{variable}': '{value}' is not a valid boolean")]
    MalformedBoolEnvVariable { variable: String, value: String },

    #[error("the value provided for environmental duration variable '{variable}': '{value}' is not a valid duration: {source}")]
    MalformedDurationEnvVariable {
        variable: String,
        value: String,
        #[source]
        source: humantime::DurationError,
    },

    #[error("the value provided for environmental numerical variable '{variable}': '{value}' is not a valid number: {source}")]
    MalformedNumberEnvVariable {
        variable: String,
        value: String,
        #[source]
        source: ParseIntError,
    },
}
