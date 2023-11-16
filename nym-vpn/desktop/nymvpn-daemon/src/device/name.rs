use std::process::Command;

pub fn command_stdout_lossy(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .ok()
}

pub fn logged_in_user() -> Option<String> {
    #[cfg(target_os = "linux")]
    let output = command_stdout_lossy("last", &["-n", "100"]);
    #[cfg(target_os = "macos")]
    let output = command_stdout_lossy("last", &["-100"]);
    #[cfg(target_os = "windows")]
    let output: Option<String> = None;

    let mut found_name = None;

    if let Some(output) = output {
        for line in output.lines() {
            if line.contains("logged in") {
                if let Some(name) = line.split_whitespace().nth(0) {
                    found_name = Some(name.to_string());
                    break;
                }
            }
        }
    }

    found_name
}

pub fn hostname() -> Option<String> {
    command_stdout_lossy("hostname", &[])
}

pub fn device_name() -> String {
    logged_in_user()
        .or_else(hostname)
        .map(|user| {
            if user.is_empty() {
                "[unknown]".into()
            } else {
                user
            }
        })
        .unwrap_or("[unknown]".into())
}

pub fn device_version() -> String {
    talpid_platform_metadata::version()
}
