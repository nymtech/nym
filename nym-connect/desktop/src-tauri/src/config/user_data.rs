use eyre::{eyre, Context, Result};
use log::error;
use serde::{Deserialize, Serialize};
use std::{fs, str};
use tauri::api::path::data_dir;

const DATA_DIR: &str = "nym-connect";
const DATA_FILE: &str = "user-data.toml";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserData {
    pub monitoring: Option<bool>,
}

impl UserData {
    pub fn read() -> Result<Self> {
        let mut data_path = data_dir().ok_or(eyre!("Failed to retrieve data directory"))?;

        data_path.push(DATA_DIR);
        data_path.push(DATA_FILE);
        let content = fs::read(&data_path).context(format!(
            "Failed to read user data {}",
            data_path.display()
        ))?;

        toml::from_str::<UserData>(str::from_utf8(&content)?).map_err(|e| {
            error!("{}", e);
            eyre!("{e}")
        })
    }

    pub fn write(&self) -> Result<()> {
        let mut data_path = data_dir().ok_or(eyre!("Failed to retrieve data directory"))?;

        data_path.push(DATA_DIR);
        data_path.push(DATA_FILE);
        let toml = toml::to_string(&self)?;
        fs::write(data_path, toml)?;
        Ok(())
    }
}
