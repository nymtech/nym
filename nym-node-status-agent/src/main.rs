use clap::Parser;

use crate::cli::Cli;

mod cli;
mod testrun;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let cli = Cli::parse();

    cli.execute().await?;

    Ok(())
}

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Directive, EnvFilter};

pub(crate) fn setup_tracing() {
    fn directive_checked(directive: impl Into<String>) -> Directive {
        directive
            .into()
            .parse()
            .expect("Failed to parse log directive")
    }

    let log_builder = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false);

    let mut filter = EnvFilter::builder()
        // if RUST_LOG isn't set, set default level
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    // these crates are more granularly filtered
    let filter_crates = [
        "reqwest",
        "rustls",
        "hyper",
        "sqlx",
        "h2",
        "tendermint_rpc",
        "tower_http",
        "axum",
    ];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{}=warn", crate_name)));
    }

    filter = filter.add_directive(directive_checked("nym_bin_common=debug"));
    filter = filter.add_directive(directive_checked("nym_explorer_client=debug"));
    filter = filter.add_directive(directive_checked("nym_network_defaults=debug"));
    filter = filter.add_directive(directive_checked("nym_validator_client=debug"));

    log_builder.with_env_filter(filter).init();
}
