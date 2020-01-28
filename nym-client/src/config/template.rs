use super::Config;
use handlebars::{Handlebars, TemplateRenderError};

fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in mod.rs.
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base client config options #####

[client]
# Human readable ID of this particular client.
id = "{{ client.id }}"

# URL to the directory server.
directory_server = "{{ client.directory_server }}"

# Path to file containing private identity key.
private_identity_key_file = "{{ client.private_identity_key }}"

# Path to file containing public identity key.
public_identity_key_file = "{{ client.public_identity_key }}"

##### additional client config options #####

# ID of the provider to which the client should send messages.
provider_id = "{{ client.provider_id }}"

# directory for mixapps, such as a chat client, to store their app-specific data.
mixapps_directory = "{{ client.mix_apps_directory }}"

##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_home_directory = "{{ client.home_directory }}"
    "#
}

fn render_template(config: &Config) -> Result<String, TemplateRenderError> {
    let reg = Handlebars::new();
    reg.render_template(config_template(), &config)
}

#[cfg(test)]
mod config_template {
    use super::*;

    #[test]
    fn it_works_for_default_config() {
        render_template(&Default::default()).unwrap();
    }

    #[test]
    fn it_works_for_dummy_config() {
        let dummy_cfg = Config {
            client: crate::config::Client {
                id: "foomp".to_string(),
            },
        };
        let render = render_template(&dummy_cfg).unwrap();

        println!("{}", render);
    }
}
