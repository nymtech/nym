// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::ValueEnum;
use std::fmt::{Display, Formatter};

#[derive(Default, Copy, Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

impl OutputFormat {
    pub fn is_text(&self) -> bool {
        matches!(self, OutputFormat::Text)
    }

    #[cfg(feature = "output_format")]
    pub fn format<T: serde::Serialize + ToString>(&self, data: &T) -> String {
        match self {
            OutputFormat::Text => data.to_string(),
            OutputFormat::Json => serde_json::to_string(data).unwrap(),
        }
    }

    #[cfg(feature = "output_format")]
    pub fn to_stdout<T: serde::Serialize + ToString>(&self, data: &T) {
        println!("{}", self.format(data))
    }

    #[cfg(feature = "output_format")]
    pub fn to_stderr<T: serde::Serialize + ToString>(&self, data: &T) {
        eprintln!("{}", self.format(data))
    }
}
