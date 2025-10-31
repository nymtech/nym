// Copyright 2016-2024 Mullvad VPN AB. All Rights Reserved.
// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{error::Error, fmt, fmt::Write};
use tracing::error;

/// Used to generate string representations of error chains.
pub trait ErrorExt {
    /// Creates a string representation of the entire error chain.
    fn display_chain(&self) -> String;

    /// Like [Self::display_chain] but with an extra message at the start of the chain.
    fn display_chain_with_msg<S: AsRef<str>>(&self, msg: S) -> String;
}

impl<E: Error> ErrorExt for E {
    fn display_chain(&self) -> String {
        let mut s = format!("Error: {self}");
        let mut source = self.source();
        while let Some(error) = source {
            if let Err(err) = write!(&mut s, "\nCaused by: {error}") {
                error!("error formatting failure: {err}");
            }
            source = error.source();
        }
        s
    }

    fn display_chain_with_msg<S: AsRef<str>>(&self, msg: S) -> String {
        let mut s = format!("Error: {}\nCaused by: {}", msg.as_ref(), self);
        let mut source = self.source();
        while let Some(error) = source {
            if let Err(err) = write!(&mut s, "\nCaused by: {error}") {
                error!("error formatting failure: {err}");
            }
            source = error.source();
        }
        s
    }
}

#[macro_export]
macro_rules! trace_err_chain {
    ($err:expr) => {
        tracing::error!("{}", $crate::ErrorExt::display_chain(&$err));
    };
    ($err:expr, $($args:tt)*) => {
        tracing::error!("{}", $crate::ErrorExt::display_chain_with_msg(&$err, ::std::format!($($args)*)));
    };
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use std::{io, path::PathBuf};

    #[test]
    #[traced_test]
    fn test_trace_err_chain() {
        trace_err_chain!(io::Error::other("file not found"));
        assert!(logs_contain("Error: file not found"));
    }

    #[test]
    #[traced_test]
    fn test_trace_err_chain_with_msg() {
        trace_err_chain!(io::Error::other("file not found"), "failed to open file");
        assert!(logs_contain("Error: failed to open file"));
        // todo: fix once it supports multiline messages
        // https://github.com/dbrgn/tracing-test/issues/48
        // assert!(logs_contain("Caused by: file not found"));
    }

    #[test]
    #[traced_test]
    fn test_trace_err_chain_with_msgfmt() {
        trace_err_chain!(
            io::Error::other("file not found"),
            "failed to open file: {}",
            PathBuf::from("test.txt").display()
        );
        assert!(logs_contain("Error: failed to open file: test.txt"));
        // todo: fix once it supports multiline messages
        // https://github.com/dbrgn/tracing-test/issues/48
        // assert!(logs_contain("Caused by: file not found"));
    }
}
#[derive(Debug)]
pub struct BoxedError(Box<dyn Error + 'static + Send>);

impl fmt::Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for BoxedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl BoxedError {
    pub fn new(error: impl Error + 'static + Send) -> Self {
        BoxedError(Box::new(error))
    }
}
