# zkNym Generation and Usage: High Level Overview

```admonish info
Access to the Mixnet is - at the time of publication - free for everyone. However, soon™ it will be required for each connecting client to present a valid credential - a zkNym - to their ingress Gateway to access the Mixnet. This document outlines the payment flow and zkNym generation for zkNyms.

zkNym access will vary depending on use:
- individual developers will have access to something like a faucet for credentials.
- larger application integrations will have their own 'under the hood' credential generation and distribution scheme for importing credentials into apps automatically.
- and NymVPN users will have a variety of payment methods avaliable to them.
_More on this soon_.
```

Generation of zkNyms involves the following actors / pieces of infrastructure:
- [NymAPI](https://nymtech.net/operators/nodes/nym-api.html) instances working together on Distributed Key Generation, referred to as the **NymAPI Quorum**. Members of the Quorum are a subset of the Nyx chain Validator set, and are part of a multisig used for triggering reward payouts to the Network Infrastructure Node Operators.
- **zkNym Requester** represented by a Bech32 address on the Nyx blockchain. This Requester might be a single user using the NymVPN app, or represent a company purchasing zkNyms to distribute to their application users, in the instance of an app integrating a Mixnet client via one of the SDKs.
- **OrderAPI**: an API creating crypto/fiat <> NYM swaps and then depositing NYM in a smart contract managed by the NymAPI Quroum for payment verification. Implementation details of the API will be released in the future.

Generation happens in 3 distinct stages:
- Key Generation & Payment
- Deposit NYM tokens & issue credential
- Generate unlinkable zkNyms for Nym Network access

The vast majority of this - from the perspective of the Requester - happens under the hood, but results in the creation and usage of an **unlinkable, rerandomisable anonymous proof-of-payment credential** - a zkNym - with which to access the Mixnet without fear of doxxing themselves via linking app usage and payment information. The user experience is further enhanced by the fact that a single credential can be split into multiple small zkNyms, meaning that a Requester may buy a large chunk of bandwidth but 'spend' this in the form of multiple zkNyms with different ingress Gateways. Whilst this happens under the hood, what it affords the Requester is an ease of experience in that they have to 'top up' their bandwidth less and are able to chop and change ingress points to the Nym Network as they see fit, akin to the UX of most modern day VPNs and dVPNs.

TODO ADD A BIG DIAGRAM FOR EACH STAGE

## Key Generation & Payment
- A Cosmos [Bech32 address](https://docs.cosmos.network/main/build/spec/addresses/bech32) is created for the Requester.
This is used to identify themselves when interacting with the OrderAPI via signed authentication tokens. This is the only identity that the OrderAPI is able to see, and is not able to link this to generated zkNyms. This identity never leaves the Requester’s device and there is no email or any personal details needed for signup. If a Requester is simply 'topping up' their subscription, the creation of the address is skipped as it already exists.
- The Requester can then interact with various payment backends to pay for their zkNyms with non-NYM crypto, fiat options, or natively with NYM tokens.
- Payment options will trigger the OrderAPI. This will:
  - Create a swap for <PAYMENT_AMOUNT> <> NYM tokens.
  - Deposit these tokens with the NymAPI Quorum via a CosmWasm smart contract deployed on the Nyx blockchain.
- The Requester generates an ed25519 keypair: this is used to identify and authenticate them in the case of using zkNyms across several devices as an individual user. However, this is never used in the clear: these keys are used as private attribute values within generated credentials which are verified via zero-knowledge.
- The Requester sends a request to each member of the Quorum requesting a credential. This request is signed with their private key and includes the transaction hash of the NYM deposit into the deposit contract, performed either by themselves or the OrderAPI. _(( TODO double check which keypair and make clear ))_

## Deposit NYM & Issue zkNym
- Once NYM tokens have been deposited into the contract controlled by the Quorum's multisig and a credential is requested, each member of the Quroum performs several checks to verify the request is valid:
  - They verify the signature sent as part of the request is valid and that the request was made in the last 48 hours.
  - They verify that the amount requested matches the amount deposited in the transcation, the hash of which was signed and sent as part of the request.
- Each member then creates a partial blinded signature - a 'partial signed credential' ('PSC') - from their fragment of the master key generated and split amongst them at the beginning of the Quroum in the initial DKG ceremony.
  - The member also creates a `key:value` entry in their local cache with the transaction hash as the key, and the PSC + encrypted signature as the value. This is used later for zkNym validation and is cleaned after a predefined timeout.
- These PSCs are given back to the Requester after setting up a secure channel via DH key ex., with each replying Quorum member also sending their public key for verification that the returned PSC was signed by them.

> In other words, each member of the Quorum who responds to the Requester's request for a zkNym (since this is a threshold cryptsystem, not all members of the Quroum must respond to create a credential, only enough to pass the threshold) returns a PSC signed with part of the master key.

- Once the Requester has received > threshold number of PSCs they can assemble them into a credential signed by the master key. The Requester never learns this master key (it is a private attribute) but the credential can be verified by the Quroum as being valid by checking for a proof that the credential's private attribute - the value of the master key - is valid.
- This credential is fed into the Requester's local 'zkNym Generator'.

## Access Network
- The zkNym Generator is entirely offline and holds the credential created from the aggregated threshold PSCs returned from individual members of the Quorum. Each time an application requests an access credential, the Generator will provide an unlinkable and unique zkNym to the requesting ingress Gateway.
- _((TODO add a point on what spend is in other terms))_
- This zkNym is later presented to the Quorum by the Gateway that collected it, which is used to calculate reward percentages given to Nym Network infrastructure operators by the Quorum, with payouts triggered by their multisig wallet.
