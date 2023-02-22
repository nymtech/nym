use crate::{manager::SentError, TaskManager};

#[cfg(unix)]
pub async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM channel");
    let mut sigquit = signal(SignalKind::quit()).expect("Failed to setup SIGQUIT channel");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
        },
        _ = sigterm.recv() => {
            log::info!("Received SIGTERM");
        }
        _ = sigquit.recv() => {
            log::info!("Received SIGQUIT");
        }
    }
}

#[cfg(not(unix))]
pub async fn wait_for_signal() {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
        },
    }
}

#[cfg(unix)]
pub async fn wait_for_signal_and_error(shutdown: &mut TaskManager) -> Result<(), SentError> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM channel");
    let mut sigquit = signal(SignalKind::quit()).expect("Failed to setup SIGQUIT channel");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
            Ok(())
        },
        _ = sigterm.recv() => {
            log::info!("Received SIGTERM");
            Ok(())
        }
        _ = sigquit.recv() => {
            log::info!("Received SIGQUIT");
            Ok(())
        }
        Some(msg) = shutdown.wait_for_error() => {
            log::info!("Task error: {:?}", msg);
            Err(msg)
        }
    }
}

#[cfg(not(unix))]
pub async fn wait_for_signal_and_error(shutdown: &mut TaskManager) -> Result<(), SentError> {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
            Ok(())
        },
        Some(msg) = shutdown.wait_for_error() => {
            log::info!("Task error: {:?}", msg);
            Err(msg)
        }
    }
}
