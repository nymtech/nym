use clap::{Args, Subcommand};

pub mod announce;
pub mod delete;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsService {
    #[clap(subcommand)]
    pub command: MixnetOperatorsServiceCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsServiceCommands {
    /// Announce service provider to the world
    Announce(announce::Args),
    /// Delete entry for service provider from the directory
    Delete(delete::Args),
}
