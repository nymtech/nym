use tracing::error;

pub(crate) async fn run_probe(identity_key: Option<String>) -> anyhow::Result<String> {
    let nym_gateway_probe_cli_path =
        std::env::var("NYM_GATEWAY_PROBE").unwrap_or("./nym-gateway-probe".to_string());
    let log = run_gateway_probe_and_get_log(nym_gateway_probe_cli_path, identity_key);

    return Ok(log)
}

fn run_gateway_probe_and_get_log(
    nym_gateway_probe_cli_path: String,
    identity_key: Option<String>,
) -> String {
    let mut command = std::process::Command::new(nym_gateway_probe_cli_path);
    command.stdout(std::process::Stdio::piped());

    if let Some(identity_key) = identity_key {
        command.arg("--gateway").arg(identity_key);
    }

    match command.spawn() {
        Ok(child) => {
            if let Ok(output) = child.wait_with_output() {
                return String::from_utf8(output.stdout)
                    .unwrap_or("Unable to get log from test run".to_string());
            }
            "Unable to get log from test run".to_string()
        }
        Err(e) => {
            error!("Failed to spawn test: {}", e);
            "Failed to spawn test run task".to_string()
        }
    }
}
