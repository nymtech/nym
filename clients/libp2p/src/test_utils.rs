// This section instantiates docker containers of the nym-client
// so that tests can be run with all the necessary resources.
// This removes the requirement for having to limit test threads
// or to build/run nym-client ourselves.

use testcontainers::{clients::Cli, core::WaitFor, images::generic::GenericImage, Container};

/// Create a nym client using the same docker Cli
pub fn create_nym_client<'a>(
    docker_client: &'a Cli,
    nym_id: &str,
) -> (Container<'a, GenericImage>, String) {
    let nym_ready_message = WaitFor::message_on_stderr("Client startup finished!");
    let nym_image = GenericImage::new("chainsafe/nym", "1.1.12")
        .with_env_var("NYM_ID", nym_id)
        .with_wait_for(nym_ready_message)
        .with_exposed_port(1977);
    let nym_container = docker_client.run(nym_image);
    let nym_port = nym_container.get_host_port_ipv4(1977);
    let nym_uri = format!("ws://0.0.0.0:{nym_port}");
    (nym_container, nym_uri)
}
