// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::errors::Result;
use log::error;
use nym_credentials::coconut::bandwidth::IssuanceBandwidthCredential;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct RecoveryStorage {
    recovery_dir: PathBuf,
}

impl RecoveryStorage {
    pub fn new(recovery_dir: PathBuf) -> Result<Self> {
        create_dir_all(&recovery_dir)?;
        Ok(Self { recovery_dir })
    }

    pub fn unconsumed_vouchers(&self) -> Result<Vec<IssuanceBandwidthCredential>> {
        let entries = read_dir(&self.recovery_dir)?;

        let mut paths = vec![];
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                paths.push(path)
            }
        }

        let mut vouchers = vec![];
        for path in paths {
            if let Ok(mut file) = File::open(&path) {
                let mut buff = Vec::new();
                if file.read_to_end(&mut buff).is_ok() {
                    match IssuanceBandwidthCredential::try_from_recovered_bytes(&buff) {
                        Ok(voucher) => vouchers.push(voucher),
                        Err(err) => {
                            error!("failed to parse the voucher at {}: {err}", path.display())
                        }
                    }
                }
            }
        }

        Ok(vouchers)
    }

    pub fn insert_voucher(&self, voucher: &IssuanceBandwidthCredential) -> Result<PathBuf> {
        todo!()
        // let file_name = voucher.tx_hash().to_string();
        // let file_path = self.recovery_dir.join(file_name);
        // let mut file = File::create(&file_path)?;
        // let buff = voucher.to_bytes();
        // file.write_all(&buff)?;
        //
        // Ok(file_path)
    }

    pub fn remove_voucher(&self, file_name: String) -> Result<()> {
        let file_path = self.recovery_dir.join(file_name);
        Ok(std::fs::remove_file(file_path)?)
    }
}
