use tracing::{debug, error, info};

pub(crate) struct GwProbe {
    path: String,
}

impl GwProbe {
    pub(crate) fn new(probe_path: String) -> Self {
        Self { path: probe_path }
    }

    pub(crate) async fn version(&self) -> String {
        debug!("Attempting to execute binary at: {}", &self.path);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            match tokio::fs::metadata(&self.path).await {
                Ok(metadata) => {
                    let perms = metadata.permissions();
                    let mode = perms.mode();
                    if mode & 0o111 == 0 {
                        error!(
                            "Binary is not executable: {} (mode: {:o})",
                            &self.path, mode
                        );
                        return "Binary is not executable".to_string();
                    }
                    debug!("Binary exists with permissions: {:o}", mode);
                }
                Err(e) => {
                    error!("Failed to stat binary at {}: {}", &self.path, e);
                    return format!("Failed to access binary: {}", e);
                }
            }
        }

        let mut command = tokio::process::Command::new(&self.path);
        command.stdout(std::process::Stdio::piped());
        command.arg("--version");

        info!("Executing command: {:?} --version", &self.path);

        match command.spawn() {
            Ok(child) => match child.wait_with_output().await {
                Ok(output) => {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .unwrap_or_else(|_| "Unable to parse version output".to_string())
                    } else {
                        let stderr = String::from_utf8(output.stderr)
                            .unwrap_or_else(|_| "Unable to parse error output".to_string());
                        error!(
                            "Command failed with exit code {}: {}",
                            output.status.code().unwrap_or(-1),
                            stderr
                        );
                        format!("Command failed: {}", stderr)
                    }
                }
                Err(e) => {
                    error!("Failed to get command output: {}", e);
                    format!("Failed to get command output: {}", e)
                }
            },
            Err(e) => {
                error!("Failed to spawn process: {}", e);
                format!("Failed to spawn process: {}", e)
            }
        }
    }

    pub(crate) fn run_and_get_log(
        &self,
        gateway_key: &Option<String>,
        mnemonic: &str,
        probe_extra_args: &Vec<String>,
    ) -> String {
        let mut command = std::process::Command::new(&self.path);
        command.stdout(std::process::Stdio::piped());

        if let Some(gateway_id) = gateway_key {
            command.arg("--gateway").arg(gateway_id);
        }
        command.arg("--mnemonic").arg(mnemonic);

        tracing::info!("Extra args for the probe:");
        for arg in probe_extra_args {
            let mut split = arg.splitn(2, '=');
            let name = split.next().unwrap_or_default();
            let value = split.next().unwrap_or_default();
            tracing::info!("{} {}", name, value);

            command.arg(format!("--{name}")).arg(value);
        }

        match command.spawn() {
            Ok(child) => {
                if let Ok(output) = child.wait_with_output() {
                    if !output.status.success() {
                        let out = String::from_utf8_lossy(&output.stdout);
                        let err = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("Probe exited with {:?}:\n{}\n{}", output.status, out, err);
                    }

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
