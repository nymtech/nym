# Basic Bandwidth Credential Generator 

This directory contains the contract and unit tests for the `BandwidthGenerator` smart contract. 

This contract allows users to generate Basic Bandwidth Credentials (BBCs) on the Nym cosmos blockchain using ERC20 representations of NYM as payment, utilising the Cosmos Gravity Bridge for cross-chain payment. 
         
BBCs are credentials that will be presented to Gateways by a Nym Client, and represent a certain amount of bandwidth which can be sent through the Nym Mixnet. 

By default 1 NYM = 1 GB of bandwidth. The ratio of NYM - bandwidth is denominated in bytes, and represented in the smart contract by the `BytesPerToken` variable. This variable can be adjusted by the contract owner. 
  
The amount of bandwidth bought is calculated according to the following formula: 
`(Token amount in 'wei' * BytesPerToken) / 10**18`

## Usage 
* `npm install`
* `npx hardhat compile`
* `npx hardhat test` 
