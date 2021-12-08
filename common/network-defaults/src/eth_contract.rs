// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// This should be modified whenever an updated Ethereum contract is uploaded
pub const ETH_JSON_ABI: &str = r#"
[
     {
       "inputs": [
         {
           "internalType": "contract CosmosERC20",
           "name": "_erc20",
           "type": "address"
         },
         {
           "internalType": "contract Gravity",
           "name": "_gravityBridge",
           "type": "address"
         }
       ],
       "stateMutability": "nonpayable",
       "type": "constructor"
     },
     {
       "anonymous": false,
       "inputs": [
         {
           "indexed": false,
           "internalType": "uint256",
           "name": "Bandwidth",
           "type": "uint256"
         },
         {
           "indexed": true,
           "internalType": "uint256",
           "name": "VerificationKey",
           "type": "uint256"
         },
         {
           "indexed": false,
           "internalType": "bytes",
           "name": "SignedVerificationKey",
           "type": "bytes"
         },
         {
           "indexed": true,
           "internalType": "bytes32",
           "name": "CosmosRecipient",
           "type": "bytes32"
         }
       ],
       "name": "BBCredentialPurchased",
       "type": "event"
     },
     {
       "anonymous": false,
       "inputs": [
         {
           "indexed": true,
           "internalType": "address",
           "name": "previousOwner",
           "type": "address"
         },
         {
           "indexed": true,
           "internalType": "address",
           "name": "newOwner",
           "type": "address"
         }
       ],
       "name": "OwnershipTransferred",
       "type": "event"
     },
     {
       "anonymous": false,
       "inputs": [
         {
           "indexed": true,
           "internalType": "uint256",
           "name": "NewBytesPerToken",
           "type": "uint256"
         }
       ],
       "name": "RatioChanged",
       "type": "event"
     },
     {
       "inputs": [],
       "name": "BytesPerToken",
       "outputs": [
         {
           "internalType": "uint256",
           "name": "",
           "type": "uint256"
         }
       ],
       "stateMutability": "view",
       "type": "function"
     },
     {
       "inputs": [
         {
           "internalType": "uint256",
           "name": "_amount",
           "type": "uint256"
         }
       ],
       "name": "bandwidthFromToken",
       "outputs": [
         {
           "internalType": "uint256",
           "name": "",
           "type": "uint256"
         }
       ],
       "stateMutability": "view",
       "type": "function"
     },
     {
       "inputs": [
         {
           "internalType": "uint256",
           "name": "_newBytesPerTokenAmount",
           "type": "uint256"
         }
       ],
       "name": "changeRatio",
       "outputs": [],
       "stateMutability": "nonpayable",
       "type": "function"
     },
     {
       "inputs": [],
       "name": "erc20",
       "outputs": [
         {
           "internalType": "contract CosmosERC20",
           "name": "",
           "type": "address"
         }
       ],
       "stateMutability": "view",
       "type": "function"
     },
     {
       "inputs": [
         {
           "internalType": "uint256",
           "name": "_amount",
           "type": "uint256"
         },
         {
           "internalType": "uint256",
           "name": "_verificationKey",
           "type": "uint256"
         },
         {
           "internalType": "bytes",
           "name": "_signedVerificationKey",
           "type": "bytes"
         },
         {
           "internalType": "bytes32",
           "name": "_cosmosRecipient",
           "type": "bytes32"
         }
       ],
       "name": "generateBasicBandwidthCredential",
       "outputs": [],
       "stateMutability": "nonpayable",
       "type": "function"
     },
     {
       "inputs": [],
       "name": "gravityBridge",
       "outputs": [
         {
           "internalType": "contract Gravity",
           "name": "",
           "type": "address"
         }
       ],
       "stateMutability": "view",
       "type": "function"
     },
     {
       "inputs": [],
       "name": "owner",
       "outputs": [
         {
           "internalType": "address",
           "name": "",
           "type": "address"
         }
       ],
       "stateMutability": "view",
       "type": "function"
     },
     {
       "inputs": [],
       "name": "renounceOwnership",
       "outputs": [],
       "stateMutability": "nonpayable",
       "type": "function"
     },
     {
       "inputs": [
         {
           "internalType": "address",
           "name": "newOwner",
           "type": "address"
         }
       ],
       "name": "transferOwnership",
       "outputs": [],
       "stateMutability": "nonpayable",
       "type": "function"
     }
   ]
        "#;
