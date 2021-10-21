// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// This should be modified whenever an updated Ethereum contract is uploaded
pub const ETH_JSON_ABI: &str = r#"
[
  {
    "inputs": [
      {
        "internalType": "contract ERC20Burnable",
        "name": "_erc20",
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
      }
    ],
    "name": "Burned",
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
    "inputs": [
      {
        "internalType": "uint256",
        "name": "amount",
        "type": "uint256"
      },
      {
        "internalType": "uint256",
        "name": "verificationKey",
        "type": "uint256"
      },
      {
        "internalType": "bytes",
        "name": "signedVerificationKey",
        "type": "bytes"
      }
    ],
    "name": "burnTokenForAccessCode",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "erc20",
    "outputs": [
      {
        "internalType": "contract ERC20Burnable",
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
