// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use credentials::coconut::bandwidth::BandwidthVoucher;
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

    pub fn unconsumed_vouchers(&self) -> std::io::Result<impl Iterator<Item = BandwidthVoucher>> {
        Ok(read_dir(&self.recovery_dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_file() {
                    Some(path)
                } else {
                    None
                }
            })
            .filter_map(|path| File::open(path).ok())
            .filter_map(|mut f| {
                let mut buff = Vec::new();
                if f.read_to_end(&mut buff).is_ok() {
                    Some(buff)
                } else {
                    None
                }
            })
            .filter_map(|buff| BandwidthVoucher::try_from_bytes(&buff).ok()))
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
