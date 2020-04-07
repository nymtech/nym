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
id = '{{ validator.id }}'

# Completely optional value specifying geographical location of this particular node.
# Currently it's used entirely for debug purposes, as there are no mechanisms implemented
# to verify correctness of the information provided. However, feel free to fill in
# this field with as much accuracy as you wish to share.
location = '{{ validator.location }}'

##### advanced configuration options #####

# nym_home_directory specifies absolute path to the home nym validators directory.
# It is expected to use default value and hence .toml file should not redefine this field.
nym_root_directory = '{{ validator.nym_root_directory }}'


##### mix mining config options #####

[mix_mining]

# Directory server from which the validator will obtain initial topology.
directory_server = '{{ mix_mining.directory_server }}'

# The uniform delay every which validator are running their mix-mining procedure.
# The provided value is interpreted as milliseconds.
run_delay = {{ mix_mining.run_delay }}

# During the mix-mining process, test packets are sent through various network
# paths. This timeout determines waiting period until it is decided that the packet
# did not reach its destination.
# The provided value is interpreted as milliseconds.
resolution_timeout = {{ mix_mining.resolution_timeout }}

# Timeout for trying to establish connection to node endpoints.
# The provided value is interpreted as milliseconds.
connection_timeout =  {{ mix_mining.connection_timeout }}
    
# How many packets should be sent through each path during the mix-mining procedure.
number_of_test_packets = {{ mix_mining.number_of_test_packets }}


##### tendermint config options #####

[tendermint]



##### logging configuration options #####

[logging]

# TODO

"#
}
