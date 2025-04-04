use std::process::Command;

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    println!("Opening URL: {}", url);

    let status = match std::env::consts::OS {
        "macos" => Command::new("open").arg(&url).status(),
        "windows" => Command::new("cmd").args(["/c", "start", &url]).status(),
        "linux" => Command::new("xdg-open").arg(&url).status(),
        os => return Err(format!("Unsupported OS: {}", os)),
    };

    match status {
        Ok(exit_status) if exit_status.success() => Ok(()),
        Ok(exit_status) => Err(format!(
            "Command failed with exit code: {:?}",
            exit_status.code()
        )),
        Err(err) => Err(format!("Failed to execute command: {}", err)),
    }
}
