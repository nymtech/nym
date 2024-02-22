# Nyx Validator Rewards

<!-- Add:
- Introduction
- References to Nym fundamental papers on the topic
- Disclaimers (not final/legal stuff etc)
- make a fn in book.toml to pull the current stake so it's always up to date
- Asign it to a VAR and use the var in the text bellow
-->

## Summary

* Nyx Validators are rewarded from the Nym mixmining pool and increasingly from apps that run on the Nym mixnet, the first of which is the NymVPN
* Validators are rewarded for two different types of work: signing blocks in the Nyx chain and running the NymAPI to monitor mixnet routing and sign zk-nym credentials
* New validators can join via a NYM-to-NYX swap contract. The contract will not allow more than 1% of total stake increase per month to prevent sudden hostile takeovers. Current stake is ~53{{current_nyx_stake}} million Nyx. Rate: 1:4.8 ~ 288k NYX for 60k NYM => 0.54% voting power
* The contract will only allow swapping NYM to NYX and will **not** allow exchanging NYX back to NYM. A NYX holder who wishes to sell their NYX stake will have to do so via OTC trades.

## Validator Rewards

Nyx Validators perform two types of work for which they will be rewarded:

1. **Signing blocks in the Nyx chain**

A “block signing monitor" monitors blocks being produced on the Nyx chain and gathers the signatures present on every block. After an epoch end, the monitor will assess performance of a mixnode and distribute tokens (to the self-delegation wallet) proportional to the voting period and uptime of the validator.

2. **Running the NymAPI to monitor Mixnet routing and sign zk-nyms** (Nym’s anonymous credentials)

Validator rewards initially come from the Nym mixmining pool with additional rewards increasingly coming from paid applications running on the Nym mixnet. The first paid application is the NymVPN. Nyx validators will be rewarded for their work directly in NYM tokens to their validator self-delegation address.

1. **From mixmining pool** - at a rate of 1000 NYM per hour, of which 2/3 are distributed for signing blocks and 1/3 for zk-nyms. These are stable in NYM, and therefore will fluctuate in their $$ value depending on exchange rate.

2. **From vpn user subscriptions** - 1/3 will be distributed for signing blocks and 2/3 for zk-nyms. These are stable in $$ and fluctuate in NYM depending on exchange rate

| Source         | Block signing | NymAPI | Currency |
| :--            | --:           | --:    | :---:    |
| Mixmining pool | 2/3           | 1/3    | NYM      |
| NymVPN         | 1/3           | 2/3    | USD      |

### zk-nyms

The zk-nyms enable people to anonymously prove access rights to the upcoming NymVPN client without having to reveal payment details that might compromise their privacy. This is the first of what we imagine to be many possible use-cases for the zk-nym scheme.

### Allocation of Rewards from Nym mixmining pool

Rewards for validators will be distributed at an hourly rate from the mixmining pool. The amount is 1000 NYM per hour to be distributed among all validators (either with hourly or daily reward periodicity). The fraction of mixmining rewards received by each individual validator is proportional to its contributions to the network.

Two thirds of the available rewards (670 NYM per hour) are distributed proportionally to each validator’s share when signing blocks in the chain, for which tx fees are also received in the same proportion; while the last third (330 NYM per hour) is allocated to validators running the NymAPI, proportionally to their contribution to signing zk-nym credentials.

The rewards are stable in NYM and fluctuate in their fiat value depending on the exchange rate of NYM tokens.

## Permissionless Nyx Chain

To allow new validators to join Nyx chain a new smart contract needs to be set up to release NYX in exchange for NYM. This contract allows a limited amount of NYM tokens to be deposited per month. The deposited NYM tokens are added to the mixmining pool and thus contribute to future rewards of all nodes and validators.

Such smart contract needs two parameters:

1. the maximum amount of NYX available for purchase per month
2. the NYM-to-NYX exchange rate offered by the contract


### Maximum Amount of NYX Available for Purchase per Month

The contract will not allow more than 1% of total stake increase per month to prevent sudden hostile takeovers. Current stake is ~53{{current_nyx_stake}} million Nyx.
