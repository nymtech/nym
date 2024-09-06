use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Directive, EnvFilter};

pub(crate) fn setup_tracing_logger() {
    fn directive_checked(directive: &str) -> Directive {
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

    // RUST_LOG directives are respected
    let filter = EnvFilter::builder()
        // if RUST_LOG isn't set, set default level
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        // these crates are more granularly filtered
        .add_directive(directive_checked("nym_bin_common=warn"))
        .add_directive(directive_checked("nym_explorer_client=warn"))
        .add_directive(directive_checked("nym_network_defaults=warn"))
        .add_directive(directive_checked("nym_validator_client=warn"))
        .add_directive(directive_checked("reqwest=error"))
        .add_directive(directive_checked("rustls=error"))
        .add_directive(directive_checked("hyper=error"))
        .add_directive(directive_checked("sqlx=error"))
        .add_directive(directive_checked("h2=error"))
        .add_directive(directive_checked("tendermint_rpc=error"));

    log_builder.with_env_filter(filter).init();
}
