// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::env::Env;
use crate::error::NymvisorError;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn copy_binary<P1, P2>(source: P1, target: P2) -> Result<(), NymvisorError>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let source_path = source.as_ref();
    let target_path = target.as_ref();

    fs::copy(source_path, target_path).map_err(|source| {
        NymvisorError::DaemonBinaryCopyFailure {
            source_path: source_path.to_path_buf(),
            target_path: target_path.to_path_buf(),
            source,
        }
    })?;
    Ok(())
}

pub(crate) fn daemon_home(
    args_home: &Option<PathBuf>,
    env: &Env,
) -> Result<PathBuf, NymvisorError> {
    if let Some(home) = args_home {
        Ok(home.clone())
    } else if let Some(home) = &env.daemon_home {
        Ok(home.clone())
    } else {
        Err(NymvisorError::DaemonHomeUnavailable)
    }
}

pub(crate) fn use_logs(args_disable_logs: bool, env: &Env) -> bool {
    if args_disable_logs {
        false
    } else if let Some(disable_logs) = env.nymvisor_disable_logs {
        !disable_logs
    } else {
        true
    }
}
