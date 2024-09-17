use anyhow::anyhow;

#[derive(Debug)]
pub(crate) struct Config {
    nym_http_cache_ttl: u64,
    http_port: u16,
}

const NYM_HTTP_CACHE_SECONDS_DEFAULT: u64 = 30;
const HTTP_PORT_DEFAULT: u16 = 8000;

impl Config {
    pub(crate) fn from_env() -> Self {
        Self {
            nym_http_cache_ttl: read_env_var("NYM_HTTP_CACHE_SECONDS")
                .unwrap_or(NYM_HTTP_CACHE_SECONDS_DEFAULT.to_string())
                .parse()
                .unwrap_or(NYM_HTTP_CACHE_SECONDS_DEFAULT),
            http_port: read_env_var("HTTP_PORT")
                .unwrap_or(HTTP_PORT_DEFAULT.to_string())
                .parse()
                .unwrap_or(HTTP_PORT_DEFAULT),
        }
    }

    pub(crate) fn nym_http_cache_ttl(&self) -> u64 {
        self.nym_http_cache_ttl
    }

    pub(crate) fn http_port(&self) -> u16 {
        self.http_port
    }
}

pub(super) fn read_env_var(env_var: &str) -> anyhow::Result<String> {
    std::env::var(env_var)
        .map_err(|_| anyhow!("You need to set {}", env_var))
        .map(|value| {
            tracing::trace!("{}={}", env_var, value);
            value
        })
}
