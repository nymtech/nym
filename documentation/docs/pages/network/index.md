# Introduction
Nym's network documentation covering network architecture, node types, tokenomics, and crypto systems.

## Technical Motivations for Nym
When you send data across the internet, it can be recorded by a wide range of observers: your ISP, internet infrastructure providers, large tech companies, and governments.

Even if the content of a network request is encrypted, observers can still see that data was transmitted, its size, frequency of transmission, and gather metadata from unencrypted parts of the data (such as IP routing information). Adversaries may then combine all the leaked information to probabilistically de-anonymize users.

The Nym mixnet provides very strong security guarantees against this sort of surveillance. It _packetises_ and _mixes_ together IP traffic from many users inside the _Mixnet_. It does this by obfuscating and anonymising traffic patterns: hiding the signal in background noise. It aims to make passive network surveillance obselete by hiding who is talking to who at any one time at the _network level_; using an anonymous email service, even one paid for anonymously, is not enough to protect you from surveillance unless you are also hiding your IP and other metadata which can be used to deanonymise you over time. We are up against agencies and companies employing enourmous compute resources scraping the web for swathes of traffic information and piping this into Machine Learning algorithms.

## Mixnet TL;DR
The Mixnet is a decentralised network of nodes run by various operators in various jurisdictions around the world, who are incentivised to do so via `NYM` token rewards: cryptocurrency allows for the creation of an incentivised, decentralised privacy network.

> If you're into comparisons, the Nym mixnet is conceptually similar to other systems such as Tor, but provides improved protections against end-to-end timing attacks which can de-anonymize users. When Tor was first fielded, in 2002, those kinds of attacks were regarded as science fiction. But the future is now here.

User applications do not have to run their own node (although we do encourage people to run infrastructure if they have the skills and time), but instead connect to the Mixnet via a Nym client either running as a separate process or (more likely) embedded in an existing application.

The Mixnet (once the credentialing system is turned on in late 2024) is pay-to-play: anonymous selective disclosure zk-nym credentials are used to ensure that clients have paid for sending bandwidth through the mixnet, but in a way that is unlinkable to their payment accounts. Furthermore each time they use a credential, it can be rerandomised: each time a user spends a credential, regardless of whether they have connected before, it appears as a new credential to the Nym network. The zk-nym scheme is a combination of the Coconut and Offline Ecash credential schemes.
