# Release Cycle

Nym operators community grows in quality and quantity. With node operators and developers joining the efforts to build Mixnet more robust and scalable, testing new features, sharing integration pull requests and generally taking an active part in Nym development, more transparency on the release cycle is required.

The core team therefore established a flow with different environments:

- ***local***: Developers use their local environments for feature building
- ***canary***: Nym internal testing environment managed by Qualty Assurance team (QA)
- [***sandbox***](sandbox.md): Public testnet, including testnet NYM token available in the [faucet](sandbox.md#sandbox-token-faucet)
- ***mainnet***: Nym Mixnet - the production version of Nym network

## Release Flow

Frequency of releases to mainnet is aimed to be every ~14 days. This time time window is an optimal compromise between periodicity and qualty assurance/testing, key factors playing an essential role in the development.

| **Stage**                                 | **Environment** | **Branch**                 | **Ownership |
| :--                                       | :--             | :--                        | :--         |
| development work                          | local/canary    | feature branches           | devs        |
| cut and test release                      | canary          | release branch             | QA          |
| bug fixing                                | canary          | directly on release branch | QA & devs   |
| put release on sandbox                    | sandbox         | release -> master/develop  | QA          |
| promote release to mainnet after 3-5 days | mainnet         | master                     | QA          |

```ascii
                   ▲                          ▲
                   │                          │
                   │  merge back into develop │
      MAINNET      ├─────────────────────────►│
      easy         │                          │
      autopromotion│                          │
      ▲            │                          │
      │            │                          │
      │            │                          │◄───────────────────────────────┐
      │            │                          │                                │
      └───release  │                          │                                │
          to       x◄───────────────┐         │                                │
          sandbox  ▲                │         │◄────────────────────────┐      │
                   │   ┌────────────►         │                         │      │
                   │   │            │         │                         │      │
                   │   │ bug        │         │                         │      │
                   │   │ fix        │         │◄─────────────────┐      │      │
                   │   │            │         │                  │      │      │
                   │   │            │         │ M                │      │      │
                   │   └────────────┤         │ I                │      │      │
                   │                │         │ L                │      │      │
                   │                └─────────x E                │      │      │
                   │                  release ▲ S                │      │      │
       ^           │                  cut     │ T                │      │      │
       :           │                  ---     │ O                │      │      │
       :           │                  fixed   │ N                │      │      │
       :           │                  release │ E                │      │      │
       :           │                  every   │     feature-bob3 │      │      │
       :           │                  14 days ├──────────────────┘      │      │
       :           │                          │                         │      │
       :           │                          │                         │      │
       :           │                          │     feature-bob2        │      │
       :           │                          ├─────────────────────────┘      │
       :           │                          │                                │
       :           │                          │                                │
       :           │                          │     feature-bob1               │
       :           │                          ├────────────────────────────────┘
       :           │                          │
       :           │                          │
       :t          │                          │
       :i          │                          │
       :m          │                          │
       :e          │                          │

                master                     develop             feature branches

ENVs
┌─────────┬────────┬──────────────────────────┬─────────────────────────────────┐
│mainnet  │sandbox │ QA / canary              │ development                     │
│         │        │                          │                                 │
└─────────┴────────┴──────────────────────────┴─────────────────────────────────┘
```

### Changes & Collaboration

To track changes easily, builders and operators can visit one of the following:

- [*CHANGELOG.md*](https://github.com/nymtech/nym/blob/master/CHANGELOG.md): Raw changelog of the merged feauters in Nym's monorepo, managed by devs and QA.
- [*Changelog page*](changelog.md): A copy of *CHANGELOG.md* with more detailed explanation, testing steps and update on documentation changes, managed by devrels.

In case you want to propose changes or resolve some of the existing [issues](https://github.com/nymtech/nym/issues), start [here](https://github.com/nymtech/nym/issues/new/choose). If you want to add content to the Operators Guide, visit [this page](legal/add-content.md).

```admonish tip
Feature tickets need explicit (while concise) wording because that title is eventually added to the changelog. Keep in mind that bad ticket naming results in bad changelog.

If you want to run in the testing environment, follow our [Sandbox testnet](sandbox.md) guide.
```
