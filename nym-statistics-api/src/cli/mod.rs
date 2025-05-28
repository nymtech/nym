use clap::Parser;
use nym_bin_common::bin_info;
use std::sync::OnceLock;
use url::Url;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Clone, Debug, Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// URL for the NS API to get a network view from
    #[clap(long, env = "NYM_API_URL")]
    pub(crate) nym_api_url: Option<Url>,

    /// HTTP port on which to run statistics api.
    #[clap(long, default_value_t = 8000, env = "NYM_STATISTICS_API_HTTP_PORT")]
    pub(crate) http_port: u16,

    /// Connection url for the database.
    #[clap(long, env = "DATABASE_URL")]
    pub(crate) database_url: String,

    /// Username for the database.
    #[clap(long, env = "POSTGRES_USER")]
    pub(crate) username: String,

    /// Password for the database.
    #[clap(long, env = "POSTGRES_PASSWORD")]
    pub(crate) password: String,

    /// PgSQL port for the database.
    #[clap(long, default_value_t = 5432, env = "PGPORT")]
    pub(crate) pg_port: u16,
}
