pub(crate) fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in mod.rs.
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base mixnode config options #####

[validator]
# Human readable ID of this particular validator.
id = "{{ provider.id }}"



# nym_home_directory specifies absolute path to the home nym validators directory.
# It is expected to use default value and hence .toml file should not redefine this field.
nym_root_directory = "{{ validator.nym_root_directory }}"



##### logging configuration options #####

[logging]

# TODO


##### debug configuration options #####
# The following options should not be modified unless you know EXACTLY what you are doing
# as if set incorrectly, they may impact your anonymity.

[debug]


"#
}
