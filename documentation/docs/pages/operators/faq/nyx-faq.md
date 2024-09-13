## Validators and tokens

### What's the difference between NYM and uNYM?

1 NYM = 1 000 000 uNYM

<!--- Commenting for now as NYX is not publicly out yet
### What's the difference between NYM and NYX?
--->

### Why some Nyx blockchain operations take one hour and others are instant?

This is based on the definition in [Nym's CosmWasm](https://github.com/nymtech/nym/tree/develop/common/cosmwasm-smart-contracts) smart contracts code.

Whatever is defined as [a pending epoch event](https://github.com/nymtech/nym/blob/b07627d57e075b6de35b4b1a84927578c3172811/common/cosmwasm-smart-contracts/mixnet-contract/src/pending_events.rs#L35-L103) will get resolved at the end of the current epoch.

And whatever is defined as [a pending interval event](https://github.com/nymtech/nym/blob/b07627d57e075b6de35b4b1a84927578c3172811/common/cosmwasm-smart-contracts/mixnet-contract/src/pending_events.rs#L145-L172) will get resolved at the end of the current interval.

### Can I run a validator?

We are currently working towards building up a closed set of reputable validators. You can ask us for coins to get in, but please don't be offended if we say no - validators are part of our system's core security and we are starting out with people we already know or who have a solid reputation.

### Why is validator set entry whitelisted?

We understand that the early days of the Nyx blockchain will face possible vulnerabilities in terms of size - easy to disrupt or halt the chain if a malicious party entered with a large portion of stake. Besides that, there are some legal issues we need to address before we can distribute the validator set in a fully permissions fashion.

### Why does Nym do airdrops?

It is part of ensuring decentralisation - we need to avoid a handful of people having too much control over the token and market. Of course ideally people will stake the tokens and contribute to the project at this stage. We run surveys to better understand what people are doing with their tokens and what usability issues there are for staking. Any feedback is appreciated as it helps us improve all aspects of using the token and participating in the ecosystem.
