use task::TaskManager;

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
async fn wait_for_signal() {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
        },
    }
}

pub(crate) async fn wait_for_interrupt(mut shutdown: TaskManager) {
    wait_for_signal().await;

    log::info!("Sending shutdown");
    shutdown.signal_shutdown().ok();

    log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
    shutdown.wait_for_shutdown().await;

    log::info!("Stopping nym API");
}
