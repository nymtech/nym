#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;
use std::{fs::Permissions, io, path::Path};

use talpid_core::logging::rotate_log;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use nymvpn_config::config;

const DEBUG: &[&str] = &["talpid_core"];

const INFO: &[&str] = &[
    "h2",
    "rustls",
    "mio",
    "netlink_sys",
    "want",
    "nftnl",
    "mnl",
    "netlink_proto",
    "hyper",
    "tower",
    "tokio_util",
    "tonic",
];

const ERROR: &[&str] = &[];

const WARN: &[&str] = &[];

fn log_level(crates: &[&str], level: &str) -> String {
    crates
        .iter()
        .map(|c| format!("{c}={level}"))
        .collect::<Vec<String>>()
        .join(",")
}

pub fn init() -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    if !std::env::var("RUST_LOG").is_ok() {
        let info = log_level(INFO, "info");
        let debug = log_level(DEBUG, "debug");
        let warn = log_level(WARN, "warn");
        let error = log_level(ERROR, "error");
        let all = [info, debug, warn, error];
        let all = all.join(",");
        std::env::set_var("RUST_LOG", format!("info,{all}"));
    }

    let config = config();

    #[cfg(unix)]
    let permissions = Some(PermissionsExt::from_mode(0o755));

    #[cfg(not(unix))]
    let permissions = None;

    create_dir(config.log_dir(), permissions)?;
    rotate_log(&config.log_dir().join(config.daemon_log_filename()))?;

    let file_appender =
        tracing_appender::rolling::never(config.log_dir(), config.daemon_log_filename());
    let (file_writer, worker_guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            fmt::Layer::default()
                .with_writer(io::stdout)
                .with_writer(io::stderr),
        )
        .with(
            fmt::Layer::default()
                .with_writer(file_writer)
                .with_ansi(false),
        )
        .try_init()?;

    std::panic::set_hook(Box::new(|panic| {
        // If the panic has a source location, record it as structured fields.
        if let Some(_location) = panic.location() {
            tracing::error!(
                message = %panic,
            );
        } else {
            tracing::error!(message = %panic);
        }
    }));

    tracing::info!("RUST_LOG: {}", std::env::var("RUST_LOG").unwrap());

    Ok(worker_guard)
}

fn create_dir(path: &Path, perm: Option<Permissions>) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path)?;

    if perm.is_some() {
        let perm = perm.unwrap();
        std::fs::set_permissions(path, perm.clone())?
    }

    Ok(())
}
