use crate::cli::Args;
use crate::log_capture::LogCapture;
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, filter::Directive, prelude::*};

mod cli;
mod log_capture;
mod probe;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_capture = setup_tracing();
    let args = Args::parse();

    args.execute(log_capture).await?;

    Ok(())
}

pub(crate) fn setup_tracing() -> LogCapture {
    fn directive_checked(directive: impl Into<String>) -> Directive {
        directive
            .into()
            .parse()
            .expect("Failed to parse log directive")
    }

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
        filter = filter.add_directive(directive_checked(format!("{crate_name}=warn")));
    }

    filter = filter.add_directive(directive_checked("nym_bin_common=debug"));
    filter = filter.add_directive(directive_checked("nym_explorer_client=debug"));
    filter = filter.add_directive(directive_checked("nym_network_defaults=debug"));
    filter = filter.add_directive(directive_checked("nym_validator_client=debug"));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false);

    let log_capture = LogCapture::new();
    let capture_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(log_capture.clone());

    tracing_subscriber::registry()
        .with(stderr_layer.with_filter(filter))
        .with(capture_layer.with_filter(LevelFilter::INFO))
        .init();

    log_capture
}
