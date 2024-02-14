#[cfg(target_os = "linux")]
mod cli;
#[cfg(target_os = "linux")]
mod config;
#[cfg(target_os = "linux")]
mod connected_client_handler;
#[cfg(target_os = "linux")]
mod constants;
#[cfg(target_os = "linux")]
mod error;
#[cfg(target_os = "linux")]
mod ip_packet_router;
#[cfg(target_os = "linux")]
mod mixnet_client;
#[cfg(target_os = "linux")]
mod mixnet_listener;
#[cfg(target_os = "linux")]
mod request_filter;
#[cfg(target_os = "linux")]
mod tun_listener;
#[cfg(target_os = "linux")]
mod util;

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), error::IpPacketRouterError> {
    use clap::Parser;

    let args = cli::Cli::parse();
    nym_bin_common::logging::setup_logging();
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        nym_bin_common::logging::maybe_print_banner(clap::crate_name!(), clap::crate_version!());
    }

    cli::execute(args).await
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("This binary is currently only supported on linux");
}
