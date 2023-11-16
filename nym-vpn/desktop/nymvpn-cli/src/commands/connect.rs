use std::time::Duration;

use clap::Args;
use console::style;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use indicatif::{ProgressBar, ProgressStyle};
use tokio_stream::StreamExt;
use tonic::Request;
use nymvpn_types::{notification::Notification, vpn_session::VpnStatus};

use crate::cli::RunCommand;

use super::{error::CliError, locations::list_locations};

#[derive(Debug, Args)]
pub struct Connect {}

pub async fn start_signal_watch(message: String) {
    tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install TERM signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        println!("{}", style(message).cyan());

        std::process::exit(0);
    });
}

#[async_trait::async_trait]
impl RunCommand for Connect {
    async fn run(self) -> Result<(), CliError> {
        // get locations
        let locations = list_locations().await?;

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(&locations)
            .with_prompt("Location:")
            .interact_opt()?;

        if let Some(index) = selection {
            // start signal watcher for user interrupts
            start_signal_watch(
                "You can continue to watch status using 'nymvpn status' cli or on the app.\nTo end current session use 'nymvpn disconnect' cli or the app".into(),
            )
            .await;

            let location = locations.get(index).unwrap();

            let mut client = nymvpn_controller::new_grpc_client()
                .await
                .map_err(|_| CliError::DaemonUnavailable)?;

            let mut stream = client.watch_events(()).await?.into_inner();

            let vpn_status = client
                .connect_vpn(Request::new(location.clone().into()))
                .await
                .map(|res| res.into_inner())
                .map(VpnStatus::from)?;

            // wait while vpn becomes active
            let pb = ProgressBar::new(100);
            let mut progress = 0;
            let mut done = false;
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.blue} [{elapsed_precise}] [{bar:.cyan/blue}] {wide_msg}",
                )
                .unwrap(),
            );

            pb.set_position(progress);
            pb.set_message(format!("{}", style(vpn_status.to_string()).yellow()));
            pb.enable_steady_tick(Duration::from_secs(1));
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if let Some(event) = event.event {
                            match event {
                                nymvpn_controller::proto::daemon_event::Event::VpnStatus(
                                    vpn_status,
                                ) => {
                                    let vpn_status: VpnStatus = vpn_status.into();
                                    progress = match vpn_status {
                                        VpnStatus::Accepted(_) => 25,
                                        VpnStatus::Connected(_, _) => {
                                            done = true;
                                            100
                                        }
                                        VpnStatus::Connecting(_) => 95,
                                        VpnStatus::Disconnected => {
                                            done = true;
                                            0
                                        }
                                        VpnStatus::Disconnecting(_) => progress,
                                        VpnStatus::ServerCreated(_) => 50,
                                        VpnStatus::ServerRunning(_) => 75,
                                        VpnStatus::ServerReady(_) => 80,
                                    };
                                    pb.set_position(progress);
                                    pb.set_message(format!(
                                        "{}",
                                        style(vpn_status.to_string()).yellow()
                                    ));
                                }
                                nymvpn_controller::proto::daemon_event::Event::Notification(
                                    notification,
                                ) => {
                                    let id = notification.id.clone();
                                    if let Ok(notification) = Notification::try_from(notification) {
                                        pb.set_position(0);
                                        pb.set_message(format!(
                                            "{}",
                                            style(notification.message).red(),
                                        ));
                                    }

                                    client.ack_notification(id).await?;

                                    done = true;
                                }
                            }
                        }
                    }
                    Err(err) => Err(err)?,
                }

                if done {
                    break;
                }
            }
            pb.finish();
        }

        Ok(())
    }
}
