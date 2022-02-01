// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

fn main() {
    match option_env!("NETWORK") {
        None | Some("sandbox") => println!("cargo:rustc-cfg=network=\"sandbox\"",),
        Some("qa") => println!("cargo:rustc-cfg=network=\"qa\""),
        _ => panic!("No such network"),
    }
}
