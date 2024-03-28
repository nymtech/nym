#[cfg(target_os = "linux")]
mod cli;

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;

    let args = cli::Cli::parse();
    nym_bin_common::logging::setup_logging();
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        nym_bin_common::logging::maybe_print_banner(clap::crate_name!(), clap::crate_version!());
    }

    cli::execute(args).await?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("This binary is currently only supported on linux");
}
