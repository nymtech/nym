// Copyright 2020 Nym Technologies SA
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

use std::fmt::Formatter;
use std::{
    error::Error as StdError,
    fmt::{self, Display},
};

/// A `Result` alias where the `Err` case is `simple_socks5_requests::Error`.
pub type Result<T> = std::result::Result<T, Error>;

// type alias for local easy of use
// there's nothing fancy about it, we just ensure it implements std::error::Error and
// that it can be safely sent to another thread (`Send`) and shared between threads (`Sync`), i.e.
// &T is `Send`. In most cases those are usually true by default.
type StdThreadError = dyn StdError + Send + Sync;

/// Possible SOCKS5 serialization errors, very strongly based on std::io::Error implementation
pub struct Error {
    repr: Repr,
}

// it has a StdError as long as it has Display and Debug
impl StdError for Error {}

#[derive(Debug)]
enum Repr {
    Simple(ErrorKind),
    Custom(Box<Custom>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    InvalidRequest,

    InvalidResponse,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match &self {
            ErrorKind::InvalidRequest => "Socks5 request parsing failure",
            ErrorKind::InvalidResponse => "Socks5 response parsing failure",
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error {
            repr: Repr::Simple(kind),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        match self.repr {
            Repr::Simple(kind) => write!(f, "{}", kind.as_str()),
            Repr::Custom(ref c) => write!(f, "{}: {}", c.kind.as_str(), c.error),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.repr, f)
    }
}

#[derive(Debug)]
struct Custom {
    kind: ErrorKind,
    error: Box<StdThreadError>,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Self
    where
        E: Into<Box<StdThreadError>>,
    {
        Error {
            repr: Repr::Custom(Box::new(Custom {
                kind,
                error: error.into(),
            })),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            Repr::Custom(ref c) => c.kind,
            Repr::Simple(kind) => kind,
        }
    }
}
