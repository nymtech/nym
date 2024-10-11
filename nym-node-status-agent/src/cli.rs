use clap::Parser;
use nym_bin_common::bin_info;
use std::sync::OnceLock;

use crate::testrun;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    #[arg(short, long = "command")]
    command_to_run: String,
    // TODO dz accept keypair for identification / auth

    // TODO dz accept NSAPI address + port
}

impl Cli {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        // TODO dz register to get a task with NSAPI

        // TODO dz for now it's None, in future read it from command line
        let log = testrun::run_probe(None).await?;

        // TODO dz report task status to NSAPI

        tracing::info!("{log}");
        Ok(())
    }
}
