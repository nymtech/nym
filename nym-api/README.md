Nym API
=======

The Nym API provides multiple services to the Nym network, and is designed to be run alongside Nyxd validators. From a logical perspective, there are multiple applications, but they are bundled together for ease of deployment.


Nym Directory Caching
----------------------

The Nym directory is contained in the mixnet smart contract in the Nyx blockchain. The blockchain holds the canonical directory information about nodes, stake, locations etc. The Nym API caches this information periodically to make queries faster and more scalable. 


Nym Network Monitoring
-----------------------

The Nym API periodically sends test packets through the entire Nym mixnet to test node liveness and quality of service. 


Nym Epoch Advancement and Payment
---------------------------------

The Nym API periodically advances the epoch and triggers payment based on network monitoring measurements.


Coconut Credentials
-------------------

Coconut [[paper](https://arxiv.org/abs/1802.07344)] is a distributed cryptographic signing scheme providing a high degree of privacy for its users. You can find an overview of how to use it in the [Coconut section](https://nymtech.net/docs/overview/private-access-control/) of the Nym documentation. 

A [simple explanation](https://constructiveproof.com/posts/2020-03-24-nym-credentials-overview/) is also available in blog form. 

This project was partially funded through the NGI0 PET Fund, a fund established by NL.net with financial support from the European Commission's NGI programme, under the aegis of DG Communications Networks, Content and Technology under grant agreement No 825310.