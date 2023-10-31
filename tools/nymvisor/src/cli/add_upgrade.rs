// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    //
}

pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    println!("add upgrade");
    Ok(())
}
