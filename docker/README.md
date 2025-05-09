## Nym Validator

This contains the configuration needed to run a Nym validator using Docker

> **SECURITY**: This runs the validator as the root user inside the container for simplicity and development purposes. This setup should NOT be used in any other fashion.

### Initial Setup

Before building and running the validator, you'll need to create the required directories:

```
# Create required directories
mkdir -p data/validator data/addresses
chmod -R 777 data
```

### Running on Apple (Mx) Macs with Colima

When running on Apple Silicon Macs, we are using Colima with x86_64 emulation to properly run the validator:

1. Install Colima
```
brew install colima
```

2. Set up a Colima VM with x86_64 architecture and Rosetta support:
```
# Create a Colima VM with x86_64 architecture
colima start nym-validator --arch x86_64 --cpu 4 --memory 8 --disk 20 --vm-type=vz --vz-rosetta --mount-type=virtiofs
```

3. Build and start the validator:
```
# Make sure you're using the Colima context
docker context use colima-nym-validator

# Build the validator
docker-compose build validator

# Start the validator
docker-compose up -d validator
```

### Standard Operation (Intel/AMD x86_64 Systems)

For standard x86_64 systems:

```
# Build the validator
docker-compose build validator

# Start the validator
docker-compose up -d validator
```

The genesis validator will be initialized with the network configuration defined in the `docker-compose.yml` file.

### Managing the Validator

To check the validator logs:

```
docker logs -f validator
```

To get the admin mnemonic:

```
docker exec validator cat /root/output/node_admin_mnemonic
```

To stop the validator:

```
docker-compose down
```

### Terminating and Cleaning Up Validator Data

If you need to completely terminate your validator and remove all associated data:

```
# Stop the containers
docker-compose down

# Remove the volumes
docker-compose down -v

# Delete the data directories
rm -rf data

# If you want to start fresh, recreate the directories
mkdir -p data/validator/config data/addresses
chmod -R 777 data
```

This will completely remove all blockchain state, keys, and configuration, allowing you to start with a clean validator instance

### Using nym-cli for Smart Contract Operations

The nym-cli utility can be used to manage and execute WASM smart contracts. You can access the CLI from within the validator container:

```
docker exec -it validator ./nym-cli cosmwasm --help
```

#### Available Commands:

- **upload**: Upload a smart contract WASM blob
- **init**: Init a WASM smart contract
- **generate-init-message**: Generate an instantiate message
- **migrate**: Migrate a WASM smart contract
- **execute**: Execute a WASM smart contract method
- **raw-contract-state**: Obtain raw contract state of a cosmwasm smart contract

#### Example Usage:

To upload a contract:

```
docker exec -it validator ./nym-cli cosmwasm upload \
  --mnemonic $(cat /root/output/node_admin_mnemonic) \
  --wasm-file /path/to/contract.wasm
```

To initialize a contract:

```
docker exec -it validator ./nym-cli cosmwasm init \
  --mnemonic $(cat /root/output/node_admin_mnemonic) \
  --code-id <CODE_ID> \
  --init-msg '{"key": "value"}'
```

For more detailed options, use the help command:

```
docker exec -it validator ./nym-cli cosmwasm <COMMAND> --help
```