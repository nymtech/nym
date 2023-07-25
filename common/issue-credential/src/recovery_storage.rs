// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct RecoveryStorage {
    recovery_dir: PathBuf,
}

impl RecoveryStorage {
    pub fn new(recovery_dir: PathBuf) -> std::io::Result<Self> {
        create_dir_all(&recovery_dir)?;
        Ok(Self { recovery_dir })
    }

    pub fn unconsumed_vouchers(&self) -> std::io::Result<Vec<BandwidthVoucher>> {
        let entries = read_dir(&self.recovery_dir)?;

        let mut paths = vec![];
        for entry in entries {
            if let Some(ok_entry) = entry.ok() {
                let path = ok_entry.path();
                if path.is_file() {
                    paths.push(path)
                }            
            }
        }

        let mut vouchers = vec![];
        for path in paths {
            if let Some(mut file) = File::open(path).ok() {
                let mut buff = Vec::new();
                if file.read_to_end(&mut buff).is_ok() {
                    if let Some(voucher) = BandwidthVoucher::try_from_bytes(&buff).ok() {
                        vouchers.push(voucher)
                    }
                }        
            }
        }

        Ok(vouchers)
    }

    pub fn insert_voucher(&self, voucher: &BandwidthVoucher) -> std::io::Result<PathBuf> {
        let file_name = voucher.tx_hash().to_string();
        let file_path = self.recovery_dir.join(file_name);
        let mut file = File::create(&file_path)?;
        let buff = voucher.to_bytes();
        file.write_all(&buff)?;

        Ok(file_path)
    }

    pub fn remove_voucher(&self, file_name: String) -> std::io::Result<()> {
        let file_path = self.recovery_dir.join(file_name);
        std::fs::remove_file(file_path)
    }
}
