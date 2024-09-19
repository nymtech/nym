use crate::read_env_var;

#[derive(Debug)]
pub(crate) struct Config {
    http_port: u16,
}

const HTTP_PORT_DEFAULT: u16 = 8000;

impl Config {
    pub(crate) fn from_env() -> Self {
        Self {
            http_port: read_env_var("HTTP_PORT")
                .unwrap_or(HTTP_PORT_DEFAULT.to_string())
                .parse()
                .unwrap_or(HTTP_PORT_DEFAULT),
        }
    }

    pub(crate) fn http_port(&self) -> u16 {
        self.http_port
    }
}
