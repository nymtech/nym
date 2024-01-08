use anyhow::{anyhow, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, path::PathBuf, str};
use tauri::api::path::data_dir;
use tokio::fs;
use tracing::{debug, error, instrument};

#[derive(Debug, Clone)]
pub struct AppStorage<T>
where
    T: Serialize + DeserializeOwned + Default + fmt::Debug,
{
    pub data: T,
    pub dir_path: PathBuf,
    pub filename: String,
    pub full_path: PathBuf,
}

async fn create_directory_path(path: &PathBuf) -> Result<()> {
    let mut data_dir = data_dir().ok_or(anyhow!(
        "Failed to retrieve data directory {:?}",
        path.display()
    ))?;
    data_dir.push(path);

    fs::create_dir_all(&data_dir).await.context(format!(
        "Failed to create data directory {}",
        data_dir.display()
    ))
}

impl<T> AppStorage<T>
where
    T: Serialize + DeserializeOwned + Default + fmt::Debug,
{
    pub fn new(dir_path: PathBuf, filename: &str, data: Option<T>) -> Self {
        let mut full_path = dir_path.clone();
        full_path.push(filename);

        Self {
            data: data.unwrap_or_default(),
            dir_path,
            filename: filename.to_owned(),
            full_path,
        }
    }

    #[instrument]
    pub async fn read(&self) -> Result<T> {
        // create the full directory path if it is missing
        create_directory_path(&self.dir_path).await?;

        // check if the file exists, if not create it
        match fs::try_exists(&self.full_path).await {
            Ok(true) => {}
            _ => fs::write(&self.full_path, []).await?,
        }

        debug!("reading stored data from {}", self.full_path.display());
        let content = fs::read(&self.full_path).await.context(format!(
            "Failed to read data from {}",
            self.full_path.display()
        ))?;

        toml::from_str::<T>(str::from_utf8(&content)?).map_err(|e| {
            error!("{e}");
            anyhow!("{e}")
        })
    }

    #[instrument]
    pub async fn write(&self) -> Result<()> {
        // create the full directory path if it is missing
        create_directory_path(&self.dir_path).await?;

        debug!("writing data to {}", self.full_path.display());
        let toml = toml::to_string(&self.data)?;
        fs::write(&self.full_path, toml).await?;
        Ok(())
    }

    #[instrument]
    pub async fn clear(&self) -> Result<()> {
        // create the full directory path if it is missing
        create_directory_path(&self.dir_path).await?;

        debug!("clearing data {}", self.full_path.display());
        fs::write(&self.full_path, vec![]).await?;
        Ok(())
    }
}
