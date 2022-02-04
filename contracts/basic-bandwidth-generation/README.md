# Basic Bandwidth Credential Generator 

This directory contains the contract and unit tests for the `BandwidthGenerator` smart contract. 

This contract allows users to generate Basic Bandwidth Credentials (BBCs) on the Nym cosmos blockchain using ERC20 representations of NYM as payment, utilising the Cosmos Gravity Bridge for cross-chain payment. 
         
BBCs are credentials that will be presented to Gateways by a Nym Client, and represent a certain amount of bandwidth which can be sent through the Nym Mixnet. 

By default 1 NYM = 1 GB of bandwidth. The ratio of NYM - bandwidth is denominated in bytes, and represented in the smart contract by the `BytesPerToken` variable. This variable can be adjusted by the contract owner. 
  
The amount of bandwidth bought is calculated according to the following formula: 
`(Token amount in 'wei' * BytesPerToken) / 10**6`

This maths relies on the fact that the `CosmosERC20` token deployed by the Gravity contract has 6 decimals (instead of the standard 18), to keep balances mirrored with Cosmos. 

## Usage 
### Basic usage
* `npm install`
* `npx hardhat compile`
* `npx hardhat test` 

### Generate a credential on rinkeby testnet:
```
# deploy Test Gravity contract, which has extra mint function for ease of testing 
npx hardhat run scripts/rinkeby/deploy-test-gravity.js --network rinkeby

(TODO script following 4 commands)
# verify Gravity contract on etherscan - remember to change the values in the `gravity-args` file 
npx hardhat verify --constructor-args ./scripts/rinkeby/gravity-args.js  --network rinkeby CONTRACT_ADDRESS
# Create a token on Gravity etherscan interface with `deployERC20()` - remember to fill in the deployed contract address in `./contractAddresses.json`
# verify token contract 
# `approve()` bandwidth generator contract with amount of tokens you want to send. Remember this token has ONLY 6 DECIMALS, not 18 as is usual with Eth tokens

# deploy the bandwidthGenerator contract: 
npx hardhat run scripts/rinkeby/deploy-bandwidth-generator.js --network rinkeby

# Run the script
 npx hardhat run scripts/rinkeby/generate-bandwidth-credential.js --network rinkeby

# Check etherscan address in the console output for event logs. 
```

