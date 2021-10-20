// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use thiserror::Error;

/// A `Result` alias where the `Err` case is `coconut_rs::Error`.
pub type Result<T> = std::result::Result<T, CoconutError>;

#[derive(Error, Debug)]
pub enum CoconutError {
    #[error("Setup error: {0}")]
    Setup(String),
    #[error("encountered error during keygen")]
    Keygen,
    #[error("Issuance related error: {0}")]
    Issuance(String),
    #[error("Tried to prepare blind sign request for higher than specified number of attributes (max: {}, requested: {})", max, requested)]
    IssuanceMaxAttributes { max: usize, requested: usize },
    #[error("Interpolation error: {0}")]
    Interpolation(String),
    #[error("Aggregation error: {0}")]
    Aggregation(String),
    #[error("Unblind error: {0}")]
    Unblind(String),
    #[error("Verification error: {0}")]
    Verification(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    #[error(
        "Deserailization error, expected at least {} bytes, got {}",
        min,
        actual
    )]
    DeserializationMinLength { min: usize, actual: usize },
    #[error("Tried to deserialize {object} with bytes of invalid length. Expected {actual} < {} or {modulus_target} % {modulus} == 0")]
    DeserializationInvalidLength {
        actual: usize,
        target: usize,
        modulus_target: usize,
        modulus: usize,
        object: String,
    },
}
