use std::process::ExitCode;

use clap::Parser;

use nymvpn_daemon::{create_daemon, logging, runtime::create_runtime, shutdown::ShutdownManager};

#[derive(Debug, Parser)]
struct Cli {
    #[cfg(windows)]
    #[arg(long)]
    service: bool,
}

fn main() -> ExitCode {
    #[cfg(windows)]
    let cli = Cli::parse();
    let _guard = match logging::init() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match create_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let exit_code = match runtime.block_on(run_nymvpn(
        #[cfg(windows)]
        cli.service,
    )) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    };

    tracing::debug!("Daemon exiting {exit_code:?}");

    exit_code
}

#[cfg(windows)]
async fn run_nymvpn(service: bool) -> Result<(), String> {
    use nymvpn_daemon::system_service;
    if service {
        system_service::run()
    } else {
        run_nymvpn_inner().await
    }
}

#[cfg(unix)]
async fn run_nymvpn() -> Result<(), String> {
    run_nymvpn_inner().await
}

async fn run_nymvpn_inner() -> Result<(), String> {
    let shutdown_manager = ShutdownManager::new();

    let daemon = create_daemon(&shutdown_manager).await?;
    let daemon_handle = tokio::spawn(async move {
        daemon.run().await;
    });

    // register signal handler after broadcaster is setup for all
    shutdown_manager.register_signal_handler().await;

    if let (Err(err),) = tokio::join!(daemon_handle) {
        tracing::error!("daemon: {err}");
    };

    tracing::info!("Daemon stopped");
    Ok(())
}
