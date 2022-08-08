# NYM - CLI TOOL
## Overview
Set of simple CLI commands to perform basic actions, such as upload contract, bond mixnode, etc.
Note: to work it requires rust 1.60+ 

## The binary
    - Build the binary 
    - cargo build --release --bin validator-client-scripts
    - this will compile the binary to "your/main/nym/path/target/release"
    - the binary is named ./validator-client-scripts

## Executing commands
    -./validator-client-scripts --help
    - will produce a list of it's capabilites 

The binary takes optional args which are as follows -> OPTIONS:        
```
--config-env-file <CONFIG_ENV_FILE>
--mixnet-contract <MIXNET_CONTRACT>      
--mnemonic <MNEMONIC>                    
--nymd-url <NYMD_URL>                    
--vesting-contract <VESTING_CONTRACT>   
```
If you specify --config-env-file it will read the values from the envs/directory:
`./validator-client-scripts --config-env-file env/qa.env .....` and you don't need to supply the 
mixnet-contract nor the vesting-contract argument

An example of a command is as follows:
```
Disclaimer the amount is in UNYMs
./validator-client-scripts --config-env-file ../../envs/qa.env --nymd-url https://qa-validator.nymtech.net --mnemonic "INPUT YOUR MNEMONIC" send --amount 100000000 --recipient <NYM_ADDRESS>
```