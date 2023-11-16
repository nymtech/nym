use std::process::ExitCode;

use async_trait::async_trait;
use clap::{Parser, Subcommand};
use console::style;

use crate::commands::{
    connect::Connect, disconnect::Disconnect, error::CliError, locations::ListLocations,
    sign_in::SignIn, sign_out::SignOut, status::Status,
};

#[async_trait]
pub trait RunCommand {
    async fn run(self) -> Result<(), CliError>;
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Sign in to your https://nymvpn.net account
    SignIn(SignIn),
    /// Sign out current device
    SignOut(SignOut),
    /// Current VPN status
    Status(Status),
    /// Available locations for VPN
    Locations(ListLocations),
    /// Connect VPN
    Connect(Connect),
    /// Disconnect VPN
    Disconnect(Disconnect),
}

impl Cli {
    pub async fn run(self) -> ExitCode {
        let output = match self.command {
            Commands::SignIn(sign_in) => sign_in.run().await,
            Commands::SignOut(sign_out) => sign_out.run().await,
            Commands::Locations(list_locations) => list_locations.run().await,
            Commands::Connect(connect) => connect.run().await,
            Commands::Disconnect(disconnect) => disconnect.run().await,
            Commands::Status(status) => status.run().await,
        };

        match output {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", style(e).for_stderr().red());
                ExitCode::FAILURE
            }
        }
    }
}
