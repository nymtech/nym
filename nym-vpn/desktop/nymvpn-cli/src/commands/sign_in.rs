use async_trait::async_trait;
use clap::Args;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Password};
use nymvpn_controller::proto::SignInRequest;
use validator::Validate;

use crate::cli::RunCommand;

use super::error::CliError;

#[derive(Args, Debug, Validate)]
pub struct SignIn {
    email: Option<String>,
}

#[async_trait]
impl RunCommand for SignIn {
    async fn run(self) -> Result<(), CliError> {
        let email = match self.email {
            Some(email) => email,
            None => Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Email:")
                .interact_text()?,
        };

        if !validator::validate_email(&email) {
            return Err(CliError::InvalidArgument(format!(
                "\"{email}\" is not a valid email"
            )));
        }

        let password = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Password:")
            .interact()?;

        let mut client = nymvpn_controller::new_grpc_client()
            .await
            .map_err(|_| CliError::DaemonUnavailable)?;

        client
            .account_sign_in(SignInRequest { email, password })
            .await?;

        println!("{}", style("Successfully signed in").yellow());

        Ok(())
    }
}
