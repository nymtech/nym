// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::checkpoint::Checkpoint;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;

/// Persists and reloads the light-client-verified head. The read side feeds the
/// [`crate::anchor::checkpoint_source::StoredCheckpointProvider`]; the write side is driven by the anchor after it advances.
pub trait CheckpointStore: Send + Sync {
    /// The last persisted head, if any (and if it parses).
    fn load(&self) -> Option<Checkpoint>;

    /// Persist `checkpoint` as the current head. Best-effort: failures are logged, not returned,
    /// since a failed persist only costs a future re-walk from the seed.
    fn save(&self, checkpoint: &Checkpoint);
}

/// A file-backed [`CheckpointStore`] (JSON). Any read/parse failure is treated as "no head".
pub struct FileCheckpointStore {
    path: PathBuf,
}

impl FileCheckpointStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        FileCheckpointStore {
            path: path.as_ref().into(),
        }
    }
}

impl CheckpointStore for FileCheckpointStore {
    fn load(&self) -> Option<Checkpoint> {
        let bytes = std::fs::read(&self.path).ok()?;
        match serde_json::from_slice(&bytes) {
            Ok(checkpoint) => Some(checkpoint),
            Err(err) => {
                warn!(
                    "ignoring unparseable persisted checkpoint at {:?}: {err}",
                    self.path
                );
                None
            }
        }
    }

    fn save(&self, checkpoint: &Checkpoint) {
        let write = || -> std::io::Result<()> {
            let json = serde_json::to_vec(checkpoint)?;
            std::fs::write(&self.path, json)
        };
        if let Err(err) = write() {
            warn!("failed to persist checkpoint to {:?}: {err}", self.path);
        }
    }
}

/// An in-memory [`CheckpointStore`] for tests.
#[derive(Default)]
pub struct InMemoryCheckpointStore {
    head: Mutex<Option<Checkpoint>>,
}

impl CheckpointStore for InMemoryCheckpointStore {
    fn load(&self) -> Option<Checkpoint> {
        #[allow(clippy::unwrap_used)]
        self.head.lock().unwrap().clone()
    }

    fn save(&self, checkpoint: &Checkpoint) {
        #[allow(clippy::unwrap_used)]
        {
            *self.head.lock().unwrap() = Some(checkpoint.clone());
        }
    }
}
