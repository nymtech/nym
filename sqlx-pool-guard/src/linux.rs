// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{io, path::Path};

static PROC_SELF_FD_DIR: &str = "/proc/self/fd/";

/// Check if there are no open file descriptors for the given files.
///
/// Linux, Android: uses `/proc/self/fd/` to list open file descriptors
/// See: https://stackoverflow.com/a/59797198/351305
pub async fn check_files_closed(file_paths: &[&Path]) -> io::Result<bool> {
    let mut dir = tokio::fs::read_dir(PROC_SELF_FD_DIR).await?;

    while let Ok(Some(entry)) = dir.next_entry().await {
        if entry
            .file_type()
            .await
            .inspect_err(|e| tracing::warn!("entry.file_type() failure: {e}"))
            .is_ok_and(|entry_type| entry_type.is_symlink())
        {
            match tokio::fs::read_link(entry.path()).await {
                Ok(resolved_path) => {
                    if file_paths.contains(&resolved_path.as_ref()) {
                        return Ok(false);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read symlink: {e}");
                }
            }
        }
    }

    Ok(true)
}
