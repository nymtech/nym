use std::path::PathBuf;

use tauri::{
    api::{path::desktop_dir, shell},
    AppHandle, Manager,
};
use nymvpn_config::config;

use crate::error::Error;

pub fn copy_to_desktop_and_open(app_handle: &AppHandle, src: PathBuf) {
    if let Some(dir) = desktop_dir() {
        let filename = src.file_name().unwrap();
        let new_path = dir.join(filename);
        if std::fs::copy(&src, &new_path).is_ok() {
            let _ = shell::open(&app_handle.shell_scope(), new_path.to_str().unwrap(), None);
        } else {
            let _ = shell::open(&app_handle.shell_scope(), src.to_str().unwrap(), None);
        }
    } else {
        let _ = shell::open(&app_handle.shell_scope(), src.to_str().unwrap(), None);
    }
}

#[tauri::command]
pub async fn open_license(app_handle: AppHandle) -> Result<(), Error> {
    let config = config();
    copy_to_desktop_and_open(&app_handle, config.license_file_path());
    Ok(())
}

#[tauri::command]
pub async fn open_log_file(app_handle: AppHandle) -> Result<(), Error> {
    let config = config();
    copy_to_desktop_and_open(&app_handle, config.daemon_log_file_full_path());
    Ok(())
}
