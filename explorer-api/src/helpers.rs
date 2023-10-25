// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{Decimal, Fraction};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

pub(crate) fn best_effort_small_dec_to_f64(dec: Decimal) -> f64 {
    let num = dec.numerator().u128() as f64;
    let den = dec.denominator().u128() as f64;
    num / den
}

pub fn failed_ips_filepath() -> String {
    let home_dir = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/failed_ips.txt", home_dir)
}

pub fn append_ip_to_file(address: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .append(true)
        .create(true)
        .open(failed_ips_filepath())
    {
        writeln!(file, "{}", address).expect("Failed to write to file");
    }
}
