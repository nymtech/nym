# Nym Validator Client (Typescript)

A TypeScript client for interacting with CosmWasm smart contracts in Nym validators.

Include the Nym Validator in your project:

```
yarn add @nymproject/nym-validator-client
```

Connect to validator and make queries

```
import Validator from '@nymproject/nym-validator-client'

const main = async () => {

 const client = await Validator.connectForQuery(rpcAddress, validatorAddress, prefix, mixnetContractAddress, vestingContractAddress, denom)

 client.getBalance(address)

}

```

Connect to validator for performing actions

```
import Validator from '@nymproject/nym-validator-client'

const main = async () => {

 const client = await Validator.connect(mnemonic, rpcAddress, validatorAddress, prefix, mixnetContractAddress, vestingContractAddress, denom)

 const res = await client.send(address, [{ amount: '10000000', denom: 'unym' }]);

}

```
