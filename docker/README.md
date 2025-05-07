## Build with Docker & Docker Compose

To build the genesis validator:

```
docker-compose build validator
```

To start the genesis validator:

```
docker-compose up -d validator
```

The genesis validator will be initialised with the network configuration defined in the `docker-compose.yml` file.

### Managing the Validator

To check the validator logs:

```
docker logs validator
```

To get the admin mnemonic:

```
docker exec validator cat /home/nym/output/genesis_mnemonic
```

To stop the validator:

```
docker-compose down
```

### Using nym-cli for Smart Contract Operations

The nym-cli utility can be used to manage and execute WASM smart contracts. You can access the cli from within the validator container:

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
  --mnemonic $(cat /home/nym/output/genesis_mnemonic) \
  --wasm-file /path/to/contract.wasm
```

To initialize a contract:

```
docker exec -it validator ./nym-cli cosmwasm init \
  --mnemonic $(cat /home/nym/output/genesis_mnemonic) \
  --code-id <CODE_ID> \
  --init-msg '{"x": "x"}'
```

For more detailed options, use the help command:

```
docker exec -it validator ./nym-cli cosmwasm <COMMAND> --help
```