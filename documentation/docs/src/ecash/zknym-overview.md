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
- zkNym Requester represented by a Bech32 address on the Nyx blockchain. The Requester is represented to the NymAPI Quorum with an ed25519 identity key. This Requester might be a single user using the NymVPN app, or represent a company purchasing zkNyms to distribute to their application users, in the instance of an app integrating a Mixnet client via one of the SDKs.
- NymVPN OrderAPI: an API creating crypto/fiat <> NYM swaps and then depositing NYM with the NymAPI Quroum for payment verification.
- Nym Network: NymAPI LINK instances working together on Distributed Key Generation, refered to as the 'Nym API Quorum'. Members of the Quorum are a subset of the Nyx chain Validator set, and are part of a multisig used for triggering reward payouts to the Network Infrastructure Node Operators.

Generation happens in 3 distinct stages:
- Key Generation & Payment
- Deposit NYM tokens & issue zkNym
- Rerandomise and use zkNym for Nym Network access

The vast majority of this - from the perspective of the Requester - happens under the hood, but results in the creation and usage of an **unlinkable, rerandomisable anonymous proof of payment credential** with which to access the Mixnet without fear of doxxing themselves via linking app usage and payment information.

## Key Generation & Payment
- A Cosmos [Bech32 address](https://docs.cosmos.network/main/build/spec/addresses/bech32) is created for the Requester.
This is used to identify themselves when interacting with the NymVPN OrderAPI via signed authentication tokens. This is the only identity that the OrderAPI is able to see, and is not able to link this to generated zkNyms. This identity never leaves the Requester’s device and there is no email or any personal details needed for signup. _If a Requester is simply 'topping up' their subscription, the creation of the address is skipped as it already exists._
- The Requester can then interact with various payment backends to pay for their zkNyms with non-NYM crypto, fiat options, or natively with NYM tokens.
- Payment options will trigger the OrderAPI. This will:
  - Create a swap for <PAYMENT_AMOUNT> <> NYM tokens.
  - Deposit these tokens with the NymAPI Quorum via a CosmWasm smart contract deployed on the Nyx blockchain.
- The Requester generates an ed25519 keypair: this is used to identify and authenticate them in the case of using zkNyms across several devices as an individual user.
- The Requester sends a request to each member of the Quorum requesting a zkNym.

## Deposit NYM & Issue zkNym
- Once NYM tokens have been deposited into a CosmWasm smart contract on the Nyx blockchain controlled by the Quorum's multisig and a zkNym is requested, each member of the Quroum create a partial blinded signature from their fragment of the key generated and split amongst them at the beginning of the Quroum in the initial DKG ceremony. These blinded signatures are sent back to the Requester.
- Once enough signatures have been received (the number of received signatures is > the required threshold) they can unblind and then aggregate them into a valid zkNym. This is fed into the local 'zkNym Generator'.

## Access Network
- The zkNym Generator is entirely offline and holds the zkNym created from the aggregated threshold signatures returned from individual members of the Quorum. Each time an application requests an access credential, the Generator will rerandomise the zkNym and present this to the requesting party (the ingress Gateway for NymVPN / Mixnet-integrated app traffic).
- This zkNym is then presented to the Quorum by the ingress Gateway that collected it, which is used to calculate reward percentages given to Nym Network infrastructure operators by the Quroum, with payouts triggered by their multisig wallet.
