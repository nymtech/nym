use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Directive, EnvFilter};

pub(crate) fn setup_tracing_logger() {
    fn directive_checked(directive: String) -> Directive {
        directive.parse().expect("Failed to parse log directive")
    }

    let log_builder = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the event's target (module path)
        .with_target(false);

    let mut filter = EnvFilter::builder()
        // if RUST_LOG isn't set, set default level
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    // these crates are more granularly filtered
    let filter_crates = [
        "nym_bin_common",
        "nym_explorer_client",
        "nym_network_defaults",
        "nym_validator_client",
        "reqwest",
        "rustls",
        "hyper",
        "sqlx",
        "h2",
        "tendermint_rpc",
    ];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{}=warn", crate_name)));
    }

    log_builder.with_env_filter(filter).init();
}
