pub(crate) fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in mod.rs.
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base mixnode config options #####

[mixnode]
# Human readable ID of this particular mixnode.
id = "{{ mixnode.id }}"

# URL to the directory server.
directory_server = "{{ mixnode.directory_server }}"

# Path to file containing private identity key.
private_identity_key_file = "{{ mixnode.private_identity_key_file }}"

# Path to file containing public identity key.
public_identity_key_file = "{{ mixnode.public_identity_key_file }}"

##### additional mixnode config options #####

    
##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = "{{ mixnode.nym_root_directory }}"


##### logging configuration options #####

[logging]

# TODO


##### debug configuration options #####
# The following options should not be modified unless you know EXACTLY what you are doing
# as if set incorrectly, they may impact your anonymity.

[debug]

"#
}
