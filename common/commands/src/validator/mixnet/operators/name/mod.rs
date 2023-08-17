use clap::{Args, Subcommand};

pub mod delete;
pub mod register;
pub mod register_sign_payload;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsName {
    #[clap(subcommand)]
    pub command: MixnetOperatorsNameCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsNameCommands {
    /// Register a name alias for a nym address
    Register(register::Args),
    /// Delete name alias for a nym address
    Delete(delete::Args),
    /// Create base58-encoded payload required for producing valiid register signature.
    CreateNameRegisterPayload(register_sign_payload::Args),
}
