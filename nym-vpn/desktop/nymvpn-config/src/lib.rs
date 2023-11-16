#[cfg(unix)]
use std::str::FromStr;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to create dir {0}: {1}")]
    CreateDirError(PathBuf, std::io::Error),
    #[error("failed to set permissions ({0:?}) on ({1}): {2:?}")]
    DirPermissionError(std::fs::Permissions, PathBuf, std::io::Error),
}

static CONFIG: OnceCell<Config> = OnceCell::new();
static CONFIG_DIR: OnceCell<PathBuf> = OnceCell::new();
static LOG_DIR: OnceCell<PathBuf> = OnceCell::new();
static SOCKET_PATH: OnceCell<PathBuf> = OnceCell::new();
const CONFIG_FILENAME: &str = "nymvpn.conf.toml";

pub fn config() -> &'static Config {
    #[cfg(windows)]
    let program_data_path = PathBuf::from(std::env::var("ProgramData").unwrap_or(
        std::env::var("PROGRAMDATA").expect("missing ProgramData and PROGRAMDATA env var"),
    ));

    let config_dir = CONFIG_DIR.get_or_init(|| {
        #[cfg(unix)]
        return PathBuf::from("/etc/nymvpn");
        #[cfg(windows)]
        {
            return program_data_path.join("nymvpn");
        }
    });

    let _ = LOG_DIR.get_or_init(|| {
        #[cfg(unix)]
        return PathBuf::from("/var/log/nymvpn");
        #[cfg(windows)]
        return program_data_path.join("nymvpn").join("log");
    });

    let _ = SOCKET_PATH.get_or_init(|| {
        #[cfg(unix)]
        return PathBuf::from("/var/run/nymvpn.sock");
        #[cfg(windows)]
        return PathBuf::from("//./pipe/nymvpn");
    });

    CONFIG.get_or_init(|| {
        Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(PathBuf::from(config_dir).join(CONFIG_FILENAME)))
            .merge(Toml::file(CONFIG_FILENAME))
            .merge(Env::prefixed("NYMVPN_"))
            .extract()
            .unwrap()
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    config_dir: PathBuf,
    log_dir: PathBuf,
    // todo: non string types for grpc and rest api?
    grpc_api_host_port: String,
    socket_path: PathBuf,
    daemon_log_filename: String,
    allowed_endpoint_ipv4: IpAddr,
    license_file_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_dir: CONFIG_DIR.get().unwrap().into(),
            log_dir: LOG_DIR.get().unwrap().into(),
            grpc_api_host_port: "grpcs://api.nymvpn.net:44444".into(),
            socket_path: SOCKET_PATH.get().unwrap().into(),
            daemon_log_filename: "nymvpn-daemon.log".into(),
            // IP of api.nymvpn.net
            allowed_endpoint_ipv4: IpAddr::V4(Ipv4Addr::new(168, 220, 80, 137)),
            license_file_path: None,
        }
    }
}

impl Config {
    pub fn db_dir(&self) -> PathBuf {
        self.config_dir.join("db")
    }

    pub fn db_url(&self) -> String {
        format!("sqlite://{}/nymvpn.db?mode=rwc", self.db_dir().display())
    }

    pub fn grpc_api_host_port(&self) -> &str {
        &self.grpc_api_host_port
    }

    pub fn allowed_endpoint_ipv4(&self) -> &IpAddr {
        &self.allowed_endpoint_ipv4
    }

    pub fn log_dir(&self) -> &Path {
        self.log_dir.as_path()
    }

    pub fn daemon_log_filename(&self) -> &str {
        &self.daemon_log_filename
    }

    pub fn daemon_log_file_full_path(&self) -> PathBuf {
        self.log_dir().join(self.daemon_log_filename())
    }

    pub fn socket_path(&self) -> &Path {
        return &self.socket_path;
    }

    pub fn version(&self) -> &'static str {
        env!("NYMVPN_VERSION")
    }

    pub fn license_file_path(&self) -> PathBuf {
        if self.license_file_path.is_some() {
            return self.license_file_path.clone().unwrap();
        }
        #[cfg(target_os = "linux")]
        return PathBuf::from_str("/opt/nymvpn/nymvpn-oss-licenses.html").unwrap();
        #[cfg(target_os = "macos")]
        return PathBuf::from_str(
            "/Applications/nymvpn.net/Contents/Resources/nymvpn-oss-licenses.html",
        )
        .unwrap();
        #[cfg(target_os = "windows")]
        return PathBuf::from(std::env::var("PROGRAMFILES").unwrap_or(
            std::env::var("ProgramFiles").expect("missing PROGRAMFILES and ProgramFiles env var"),
        ))
        .join("nymvpn")
        .join("nymvpn-oss-licenses.html");
    }

    pub fn icon_path(&self) -> &'static str {
        #[cfg(target_os = "linux")]
        return "/usr/share/icons/hicolor/32x32/apps/nymvpn.png";
        #[cfg(target_os = "macos")]
        return "/Applications/nymvpn.net/Contents/Resources/icon.icns";
        #[cfg(target_os = "windows")]
        return "";
    }
}
