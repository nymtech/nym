// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use sha2::Digest;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::{fs, io};
use tracing::trace;

pub(crate) fn init_path<P: AsRef<Path>>(path: P) -> Result<(), NymvisorError> {
    let path = path.as_ref();
    trace!("initialising {}", path.display());

    fs::create_dir_all(path).map_err(|source| NymvisorError::PathInitFailure {
        path: path.to_path_buf(),
        source,
    })
}

pub fn calculate_file_checksum<D: Digest, P: AsRef<Path>>(
    filepath: P,
) -> Result<Vec<u8>, io::Error> {
    let file = File::open(filepath)?;
    let mut reader = BufReader::new(file);

    let mut hasher = D::new();
    let mut buf = vec![0; 4096];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n])
    }

    Ok(hasher.finalize().to_vec())
}

pub fn to_hex_string<T: AsRef<[u8]>>(input: T) -> String {
    hex::encode(input)
}
