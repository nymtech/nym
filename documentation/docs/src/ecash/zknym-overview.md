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
- Nym Network: [NymAPI](https://nymtech.net/operators/nodes/nym-api.html) instances working together on Distributed Key Generation, referred to as the **NymAPI Quorum**. Members of the Quorum are a subset of the Nyx chain Validator set, and are part of a multisig used for triggering reward payouts to the Network Infrastructure Node Operators.
- **zkNym Requester** represented by a Bech32 address on the Nyx blockchain. This Requester might be a single user using the NymVPN app, or represent a company purchasing zkNyms to distribute to their application users, in the instance of an app integrating a Mixnet client via one of the SDKs.
- **OrderAPI**: an API creating crypto/fiat <> NYM swaps and then depositing NYM in a smart contract managed by the NymAPI Quroum for payment verification. Implementation details of the API will be released in the future.

Generation happens in 3 distinct stages:
- Key Generation & Payment
- Deposit NYM tokens & issue credential
- Generate unlinkable zkNyms for Nym Network access

The vast majority of this - from the perspective of the Requester - happens under the hood, but results in the creation and usage of an **unlinkable, rerandomisable anonymous proof-of-payment credential** - a zkNym - with which to access the Mixnet without fear of doxxing themselves via linking app usage and payment information.

## Key Generation & Payment
- A Cosmos [Bech32 address](https://docs.cosmos.network/main/build/spec/addresses/bech32) is created for the Requester.
This is used to identify themselves when interacting with the OrderAPI via signed authentication tokens. This is the only identity that the OrderAPI is able to see, and is not able to link this to generated zkNyms. This identity never leaves the Requester’s device and there is no email or any personal details needed for signup. If a Requester is simply 'topping up' their subscription, the creation of the address is skipped as it already exists.
- The Requester can then interact with various payment backends to pay for their zkNyms with non-NYM crypto, fiat options, or natively with NYM tokens.
- Payment options will trigger the OrderAPI. This will:
  - Create a swap for <PAYMENT_AMOUNT> <> NYM tokens.
  - Deposit these tokens with the NymAPI Quorum via a CosmWasm smart contract deployed on the Nyx blockchain.
- The Requester generates an ed25519 keypair: this is used to identify and authenticate them in the case of using zkNyms across several devices as an individual user. However, this is never used in the clear: these keys are used as private attribute values within generated credentials which are verified via zero-knowledge.
- The Requester sends a request to each member of the Quorum requesting a credential.

## Deposit NYM & Issue zkNym
- Once NYM tokens have been deposited into the contract controlled by the Quorum's multisig and a credential is requested, each member of the Quroum create a partial blinded signature - a 'partial signed credential' ('PSC') - from their fragment of the master key generated and split amongst them at the beginning of the Quroum in the initial DKG ceremony. These PSCs are given back to the Requester. In other words, each member of the Quorum who responds to the Requester's request for a zkNym (since this is a threshold cryptsystem, not all members of the Quroum must respond to create a credential, only enough to pass the threshold) returns a PSC signed with part of the master key.
- Once the Requester has received > threshold number of PSCs they can assemble them into a credential signed by the master key. The Requester never learns this master key (it is a private attribute) but the credential can be verified by the Quroum as being valid by checking for a proof that the credential's private attribute - the value of the master key - is valid.
- This credential is fed into the Requester's local 'zkNym Generator'.

## Access Network
- The zkNym Generator is entirely offline and holds the credential created from the aggregated threshold PSCs returned from individual members of the Quorum. Each time an application requests an access credential, the Generator will provide an unlinkable and unique zkNym to the requesting ingress Gateway.
- This zkNym is later presented to the Quorum by the Gateway that collected it, which is used to calculate reward percentages given to Nym Network infrastructure operators by the Quorum, with payouts triggered by their multisig wallet.

### zkNym Unlinkability
Each time a credential is requested by an ingress Gateway to prove that a client has purchased data to send through the Mixnet, the zkNym Generator will provide a new, unlinkable zkNym. This is a rereandomised value that is able to be verified as being legitimate (in that it was created by feeding a valid root credential into the Generator) but **not linked to any other zkNyms**, either previously generated or to be generated in the future.

```admonish info
The functionality included in the following code block examples were added to the [nym-cli tool](../tools/nym-cli.md) for illustrative purposes only: this is not necessarily how credentials will be accessed in the future.

**Furthermore, the `nym-cli` uses the words 'tickets' in place of 'zkNyms' and 'ticketbook' in place of 'aggregated credential': this was WIP internal wording that we are moving away from now.**
```

```
❯ ./nym-cli ecash generate-ticket --credential-storage storage.db --provider 6qidVK21zpHD298jdDa1RRpbRozP29ENVyqcSbm6hQrG --ticket-index=3
TICKETBOOK DATA:
4Ys9pzUf9MPxX4s5RASyrRoY9fPk1a1kFuPBP2jm2L5PyUy535yPEfjHAfpUTC1Lf2d155TmjukvcDycQYfBSDfhEUJM4J3qPNfG3B5aQEEkefESZp3CM5AEnAu1AEyhpepbYw6BuXokiNcmaYtq3yJQbA4KicKP8FowoRzKHmXpJoUqY8wYQughGfdtXgr3rVaZmK21X51P1NL2UW1aCE512WWfy6P1LJHByWywT3qVw28Z83

attempting to generate payment for ticket 3...

PAYMENT FOR TICKET 3:
VfZAuVRRHekQYMvFevNAZmPPuwMAfEhTBY8TXatBysbrNXAg8euEGPpJvdbhNfQSznBb9nRSeBUSVoNTToSA6Uj5dXmJ7oE2rCB439DarLMWHWYfQNhw6yhWJhcg6bt7ebBYTs3vVeQgSB5kYuifzJF4QQmK6uJyTNPvpV1J6V8M32PBkGT3JpVB3GUGZiksETf7TaF9wAhMo2QAMxw5ZvaQVve5ea7Mane6cfb2Gx69SRff5zDfEQvKqKnyyZje4SGZgWUeHWVLhRjg4KMTJ3JcsHxEqj2k5qeGeyBbgzcuEtCpYvaytsz7nuZGJsT4Z87gB5Zq4NGuDmekuN977eRJvua2dASNWeHiAzVyvnS7ARN5cdUjjYKYiWgHaYrHGsv26WTDeiu4U3sdJMrLHGFY5ihX7f8sTZqD6Wx5AWjQNbEtKaVHymDogfLcwGCC42gQ2yhKfPUaWJ8H4yMB65YBDXGjATaUzcDmJcZKx8g31j2uTVNSFUesd5CRNEEcTNW7cSFFCishCD3T4eV9SuyZyEXAZ48pazPzc1BysBNHEXQNUEtEAZTKmpghC2pihhfDub6LnMJPo9DDdhCULCbcWbGAPc1vPekPaWvk7wrUTGwp5xoNUhQLW3MeJzMvrMSsqLdursCKB4h4Tk272WCStCPQwAKMYoxjWvMzxoUTTWCkhLKHruMtsehRnai4vhu13jbui6ji1F389gfazm4ctth2s4Yw3H3SaPtRETBfZNvZ7n5UV1MD6Q3qin92gT65iqXEi4zRN3woYcK6ZehiSvgUksdEFAUSxNMgNXKtHEYDS6kA37tn5JdBa2Ex2jLudFfhg6JBM226ZKyj65o6feYPgbJAR3jMCmQRHe6DSFb4aH895EowNMjfGUhwhmnbYB1djp7iFXxPP7575NAerhxEQ1WFnxTfoX7pu1Vc9YZb5priCAVbATCaDkECJsdedM45Vx96Jc6E5NWqD98RhMsPimVJkSfYJmRxH9qugica6WonFFb2YLvXYyhoBA1VHBcRqZJ5KHitS5AegYSoYprUfubMzcYo2hGVEQkGKAsFq6jZgCsbJoGLXt3No317vcowB5f3hqT9FjASHAzW2j8uJ9RRzX7XtrPhArwx4EyPgYzrvgG7xcenoSgQt8poa7aYky56eZTKHVUZgUEt6St32MjcivMvmNdWiAHHDc2ZxzTJHgeuCckX7n19vQ3XNLuXv9oGKNNCi8kHnT4tUnnGXNAWXWuyBgZKWUL8u3y41iW6dLYK3Pw5zfpKZTrq3q3bTLJRN5LnnUuFVnWsC3SNqa6VAAvhTGR9PzxLk8C6HeLP2AsYPpqeQwbaL3Ks6tvPdob3tQPWRBGL4uiKtNZ23tRYZGZLYFWZK7psRSZg5AETejKxztVzAuYovpVUiDq71o331tjqWWV1SzWT13Rd1uwz6nHtsjgao2863YaizKARcYr1j9MKtNfDs483yho6i7tbCRR9M4CPLqdiKEaRyVC1FP4F3sejA6nZTuAA35JWUzX6BBj7wgdypMLdMmmtcCZm3bRrF3GvJJs67U8JWRc6dnoGUDaD7rUu
```

Now lets generate another zkNym to spend either topping up once the previous one's data allowance has been used, or with another Gateway. Notice that the `ticket-index` is the same: this is generated from the same aggregated credential as the one above!

```
❯ ./nym-cli ecash generate-ticket --credential-storage storage.db --provider 6qidVK21zpHD298jdDa1RRpbRozP29ENVyqcSbm6hQrG --ticket-index=3
TICKETBOOK DATA:
4Ys9pzUf9MPxX4s5RASyrRoY9fPk1a1kFuPBP2jm2L5PyUy535yPEfjHAfpUTC1Lf2d155TmjukvcDycQYfBSDfhEUJM4J3qPNfG3B5aQEEkefESZp3CM5AEnAu1AEyhpepbYw6BuXokiNcmaYtq3yJQbA4KicKP8FowoRzKHmXpJoUqY8wYQughGfdtXgr3rVaZmK21X51P1NL2UW1aCE512WWfy6P1LJHByWywT3qVw28Z83

attempting to generate payment for ticket 3...

PAYMENT FOR TICKET 3:
Vev3SmwWtH5vbnejX5Zzc1EcxXAgveqHpKNN8arxXaWLhFcEpdcZ6n7qr3NrQUNURWsK2AsUiX8aSiGSjMPEY3iDE3aDYnjYERVow8RKUmQiYSKvz7v9cEJxt97JAHBfu9WYNHXTnLFSJwWuFtBdzY5dzPdzGckFenGCysa1ZBHGADHChDVXKoPHXxpn5qyJxmi48coUQDptR64QgkCeQ8RRZ396Lxw2NKFSjqavCMMDVm3g1rW7cYyPanBhkoAUzPU9KXX1rtmhD6F9gV89mGZ8fm7ByDuKuYU28seLQ7GkVKkhNeRW9XxbjSiyscTnMUzJ24R5VbSdr141BaquUHezdUTzmA2EjAtcyyiVrCMV13cc96CRbMXENP2soUzckFnh1qPnrfKCvX4JYkztq7UgPT2mZEnSTDW4C6Z2NVCNBPNLqUSYrU4id8Jzcp1mBxqJjdYcQ7P5fWJbT5Q9NAq44PCgfXpsUkNoj35QVQvKXKLb5oNGqnua5YC1WBPcENcpS7ZPWpk2hwe8VK4gNgnwQtWH2RPmWbvBREAV97vS1vKNHJyry9sD2PiMJGSmBnb1bKsGxR9UQN3YvRsdGHzyJHzAMTzxbFJBqMPmxjSHJR4UdwzhB81Ludu1RAffTvecWFxmWH5bNymCQjw3wey7Uequcxgyy8KAWYDzvHGwCZQbHQXghsYREiqquZWaa8hX3iTNBFUtEk8PRVT78MoFNdeBWNjsLr8zyZ5EGnf4kqmw3a91g5p5vywf6e3LgMu19VHjPSNtKMNXiatkPEVjsCuCppmV4sB7FsdKKWcMUSWLsdmrDBg9PStHr7NaJRzLL5E91gvysmB36Nob9cHeHSZj3wM4NVVjFfZeRqQf4bi7ahfXjeeBetgDpqx7JcbU6tTN4JpcGUpp7fp4MhTq7MeVQMLweGUVLqewKgAGzCvEmrK6dzLd3U1P9vkAAVZ3cCAKUywnHGxoxDeEfexP1g1EqJLtKNZVKPf7hSMWqGhoQ36K7y5GnyZ5YhQ7jcDME9orm5w4StoxoDdCPcjbakKG7UaTHuhd7tU1mUffXcEvVerkXoQK9SEaKvGks21RBhW86aHUzJWVbkiDzdaqjJWbmzLV8FKvNxNyzucoH2rq8LiHRMZfV1H3SkVSa4j2Ktw7ZGoQfdj8DgekxXSR2nHPfhybzKYXTBqFo2ACisxkjR4rXr9Xo6eYywQhQ1MP6aYgYCAXFGHPoFf7kx7Jns5sWvHRBdaMF65zeFF2m5NDuMWETtLgFfsyNgR84vfSqTfzj2gsUykRei7q9N4LKmiDwBALTAEcTvZpLtXBjc8JaB9PUeBw7DoSiSK376sGrQ9F6ZGTngXACNz1TbvYhtau4bDa6KC2Qn7wmoyrphpn7TtM1jdwGBxLcaEEWZKQHvWVfTyL2itjqnrcAZkxYdCj56oQYwpWfKQk3zJEUA6SYHqyJjaLNVK6u25j7969EWjdpTsJ8qSsZgXi3T7dQqiwintZbUUUKRq7egN1SGVnA6Wup91uKrYUWEWMqVu4g8ipmRsLD9iXHHr3yA21Cka7pqk1FxR9BFTAnkk1
```

These are both generated by the _same_ underlying credential fed into the zkNym Generator and verified and used in a way that they cannot be tied to each other. An ingress Gateway might (for instance) get 100 connection requests from 100 Nym clients, each validated with a zkNym. It has no way of knowing whether these are all zkNyms from the same single subscription, or 100 different ones.

### Incremental Spending
Each zkNym generated by the Generator will not be valid for the entire amount of data that the credential aggregated from the PSCs is; if the aggregated credential is worth (e.g.) 10GB of Mixnet data, each zkNym created by the Generator will be worth far less.

```admonish info
The functionality included in the following code block examples were added to the [nym-cli tool](../tools/nym-cli.md) for illustrative purposes only: this is not necessarily how credentials will be accessed in the future.

**Furthermore, the `nym-cli` uses the words 'tickets' in place of 'zkNyms' and 'ticketbook' in place of 'aggregated credential': this was WIP internal wording that we are moving away from now.**

The numbers used in this high level overview are for illustration purposes only. The figures used in production will potentially vary. Note that individual zkNym sizes will be uniform across the Network.
```

This is to account for the need for a client to change their ingress Gateway, either because the Gateway itself has gone down / is not offering the required bandwidth, or because a user might simply want to split their traffic across multiple Gateways for extra privacy.

In order to accomodate this then each generated zkNym will be worth a far smaller amount than the aggregated credential fed into the zkNym Generator. A single aggregated credential worth (e.g.) 10GB of data might be split into 100MB zkNym chunks. This means that clients are not tied to particular Gateways they have 'spent' their entire subscription amount with; if the ingress Gateway goes down, or the client simply wishes to use another ingress Gateway, the user has multiple other zkNyms they can use that account for their remaining purchased bandwidth.

Going back to the `nym-cli` tool to illustrate this; we can generate multiple unlinkable zkNyms ('tickets' on this command output) from a single aggregated credential ('ticketbook' below):

```
❯ ./nym-cli ecash generate-ticket --credential-storage storage.db --provider 6qidVK21zpHD298jdDa1RRpbRozP29ENVyqcSbm6hQrG --full
TICKETBOOK DATA:
4Ys9pzUf9MPxX4s5RASyrRoY9fPk1a1kFuPBP2jm2L5PyUy535yPEfjHAfpUTC1Lf2d155TmjukvcDycQYfBSDfhEUJM4J3qPNfG3B5aQEEkefESZp3CM5AEnAu1AEyhpepbYw6BuXokiNcmaYtq3yJQbA4KicKP8FowoRzKHmXpJoUqY8wYQughGfdtXgr3rVaZmK21X51P1NL2UW1aCE512WWfy6P1LJHByWywT3qVw28Z83

generating payment information for 50 tickets. this might take a while!...
AVAILABLE TICKETS
+-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------+
| index | binary data                                                                                                                                                   | spend status       |
+============================================================================================================================================================================================+
| 0     | 4kgKyJLq1zQuk9r9AbEFHPqD8mDuxsLSjgo9XW4Lf7EqGSbgfNsWSEcTbRPEMFLzpstbX5azsA3opFh851h4g5qCG2qE3Luwqua4GG2ebJhk91rvEc5JPctbVQxL62fkfQ6svdcNp…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
| 1     | 4kefQqViRZd5YezMHH1FTcgUGPK2E2ivfmwgf59exvsnR8tsb5aJtGVwpA7wAJT6icPeo8jtDwDZ3WMPJxL3VRLiakAQr79zh7ixM89gowg3ChHEy6ewmHcT7T6RFkZFsMCMj1CNd…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
| 2     | 4kxaKdBxyFzJ8gxSZCh1v3wBfN7JvnCJuoJ4MWqkkMHtt2XgRKbDmHCv5ZxtA57Qk8LC3NDMBmqjADvY34mAPdT3tLBL4uxse9ASa227Ji96dwgxvfbpvLXSSr5o4vuPRV9K7UfpJ…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
| 3     | 4kdYwUJwXyxZBLQXextd4GsU2MATjzArVq5Ec459fTXyrm6q3vxurWULzBMpV5UjcmjJtnw1zFqt7f8Ydu5gyxwAVXP3Nwpn83ouguv2n4YrUewZCvFAqQYXgahhhaQGp6RxK2Arh…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
...

| 46    | 4kg8bfQ7kGgq5TkkqXagpAEu95gmGT4i7NKbaxJtp2gRgWRrQZM1rxaDAzAxfghoM6PFNbYgKsnLD4MF8HtXW3p92CnPBjswzJ1EbtsMGpgDER3CYFt2ivAhMAVXFziF5UjVJXhpa…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
| 47    | 4kipbH5Fqt5E9hFMynm9vzFh5FkxKRdHrSEiiJWDwmg3mASctR61sXoFD5u5ZMBwGdvz9sWsRfrpR4MX2NNfRhC85aUxqtkAv3hXZiCLtE1pUC54Cq7YXHyv2XTNKpvuFZs2GmwYg…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
| 48    | 4kxYZ26HXvxVhh4quHXeCUyQokydeF5wkwUi8fMx6P3uoMvuiPaNP1SJTbYnaQEFFtF6U4dGop6QckUYvbtwQFoGJTJesHFHTDtHbshj5Dg8DwbyaHuAR86zGwYMUPved4XKUTMLa…1057bytes remaining | NOT SPENT |
|-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------|
| 49    | 4kb6zmPebRxjKLVicctq2whvANjWJMoohiPBMr21cT4xj78nvXmJEK8EB4PpqQVFo6ddU9uzuer5ggQZNZgETX2VXBzymBYNzXBuXjLJi1WRdAiASqWz5Hv5im1TJh4XBE4mxKo8Q…1057bytes remaining | NOT SPENT |
+-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------+--------------------+
```

### Double-Spend Protection
TODO
