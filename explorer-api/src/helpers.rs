// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{Decimal, Fraction};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub(crate) fn best_effort_small_dec_to_f64(dec: Decimal) -> f64 {
    let num = dec.numerator().u128() as f64;
    let den = dec.denominator().u128() as f64;
    num / den
}

pub fn failed_ips_filepath() -> String {
    let home_dir = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let mut path = PathBuf::from(home_dir);
    path.push("failed_ips.txt");
    path.to_string_lossy().into_owned()
}

pub fn append_ip_to_file(address: &str) {
    match OpenOptions::new()
        .append(true)
        .create(true)
        .open(failed_ips_filepath())
    {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", address) {
                error!("Failed to write to file: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to open or create the file: {}", e);
        }
    }
}
