// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node::error::NymNodeError;
use nym_node_http_api::api::api_requests::v1::node::models::NodeDescription;
use std::fs;
use std::fs::create_dir_all;
use std::path::Path;

pub fn load_node_description<P: AsRef<Path>>(path: P) -> Result<NodeDescription, NymNodeError> {
    let raw = fs::read_to_string(path.as_ref()).map_err(|source| {
        NymNodeError::DescriptionLoadFailure {
            path: path.as_ref().to_path_buf(),
            source,
        }
    })?;

    toml::from_str(&raw).map_err(|source| NymNodeError::MalformedDescriptionFile { source })
}

pub fn save_node_description<P: AsRef<Path>>(
    path: P,
    description: &NodeDescription,
) -> Result<(), NymNodeError> {
    // SAFETY:
    // the unwrap is fine as our description format can be serialised as toml
    #[allow(clippy::unwrap_used)]
    let serialised = toml::to_string_pretty(description).unwrap();
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).map_err(|source| NymNodeError::DescriptionSaveFailure {
            path: path.as_ref().to_path_buf(),
            source,
        })?
    }

    fs::write(path.as_ref(), serialised).map_err(|source| NymNodeError::DescriptionSaveFailure {
        path: path.as_ref().to_path_buf(),
        source,
    })
}
