# Basic Bandwidth Credential Generator 

This directory contains the contract and unit tests for the `BandwidthGenerator` smart contract. 

This contract allows users to generate Basic Bandwidth Credentials (BBCs) on the Nyx blockchain using ERC20 NYM as payment, utilising the Gravity Bridge for cross-chain payment. 
         
By default 1 NYM = 1 GB of bandwidth. The ratio of NYM - bandwidth is denominated in bytes, and represented in the smart contract by the `BytesPerToken` variable. This variable can be adjusted by the contract owner. 
  
The amount of bandwidth bought is calculated according to the following formula: 
`(Token amount in 'wei' * BytesPerToken) / 10**18`

## Usage 
* `npm install`
* `npx hardhat compile`
* `npx hardhat test` 

If you are planning to interact with contracts on either Rinkeby or mainnet, or deploy new contract instances, create an `.env` file with the following: 

```
RINKEBY_URL=https://rinkeby.infura.io/v3/<INFURA_KEY>
MAINNET_URL=https://mainnet.infura.io/v3/<INFURA_KEY>
PRIV_KEY=<YOUR_PRIVATE_KEY>
ETHERSCAN_API_KEY=<ETHERSCAN_API_KEY>
```

## Deployed addresses
Find deployed instances of the smart contract on the Rinkeby testnet and the Ethereum mainnet in `contractAddresses.json`. 

This json file is automatically updated when any of the scripts in `scripts/mainnet` or `scripts/rinkeby` are run. 