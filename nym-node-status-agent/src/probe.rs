use tracing::error;

pub(crate) struct GwProbe {
    path: String,
}

impl GwProbe {
    pub(crate) fn new(probe_path: String) -> Self {
        Self { path: probe_path }
    }

    pub(crate) async fn version(&self) -> String {
        let mut command = tokio::process::Command::new(&self.path);
        command.stdout(std::process::Stdio::piped());
        command.arg("--version");

        match command.spawn() {
            Ok(child) => {
                if let Ok(output) = child.wait_with_output().await {
                    return String::from_utf8(output.stdout)
                        .unwrap_or("Unable to get log from test run".to_string());
                }
                "Unable to get probe version".to_string()
            }
            Err(e) => {
                error!("Failed to get probe version: {}", e);
                "Failed to get probe version".to_string()
            }
        }
    }

    pub(crate) fn run_and_get_log(&self, gateway_key: &Option<String>) -> String {
        let mut command = std::process::Command::new(&self.path);
        command.stdout(std::process::Stdio::piped());

        if let Some(gateway_id) = gateway_key {
            command.arg("--gateway").arg(gateway_id);
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
}
