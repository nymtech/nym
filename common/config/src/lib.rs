use handlebars::Handlebars;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub trait NymConfig: Default + Serialize + DeserializeOwned {
    fn template() -> &'static str;

    fn config_file_name() -> String;

    fn default_root_directory() -> PathBuf;

    // default, most probable, implementations; can be easily overridden where required
    fn default_config_directory(id: Option<&str>) -> PathBuf {
        Self::default_root_directory()
            .join(id.unwrap_or(""))
            .join("config")
    }

    fn default_data_directory(id: Option<&str>) -> PathBuf {
        Self::default_root_directory()
            .join(id.unwrap_or(""))
            .join("data")
    }

    fn root_directory(&self) -> PathBuf;
    fn config_directory(&self) -> PathBuf;
    fn data_directory(&self) -> PathBuf;

    fn save_to_file(&self, custom_location: Option<PathBuf>) -> io::Result<()> {
        let reg = Handlebars::new();
        // it's whoever is implementing the trait responsibility to make sure you can execute your own template on your data
        let templated_config = reg.render_template(Self::template(), self).unwrap();

        fs::write(
            custom_location.unwrap_or(self.config_directory().join(Self::config_file_name())),
            templated_config,
        )
    }

    fn load_from_file(custom_location: Option<PathBuf>, id: Option<&str>) -> io::Result<Self> {
        let config_contents = fs::read_to_string(
            custom_location.unwrap_or(Self::default_config_directory(id).join("config.toml")),
        )?;

        let parsing_result = toml::from_str(&config_contents)
            .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err));

        parsing_result
    }
}
