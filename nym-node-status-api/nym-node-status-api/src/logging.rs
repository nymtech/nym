use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Directive, EnvFilter};

// TODO dz you can get the tracing-subscriber via basic-tracing feature on nym-bin-common
pub(crate) fn setup_tracing_logger() -> anyhow::Result<()> {
    fn directive_checked(directive: impl Into<String>) -> anyhow::Result<Directive> {
        directive.into().parse().map_err(From::from)
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
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    // these crates are more granularly filtered
    let warn_crates = [
        "reqwest",
        "rustls",
        "hyper",
        "sqlx",
        "h2",
        "tendermint_rpc",
        "tower_http",
        "axum",
    ];
    for crate_name in warn_crates {
        filter = filter.add_directive(directive_checked(format!("{}=warn", crate_name))?);
    }

    let log_level_hint = filter.max_level_hint();

    log_builder.with_env_filter(filter).init();
    tracing::info!("Log level: {:?}", log_level_hint);

    Ok(())
}
