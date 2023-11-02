# A Note on Infrastructure
If you are writing an application that requires sending messages through the mixnet, then you will either be relying on existing infrastructure nodes (network requesters), or writing your own custom service (for example, the service written as part of the Rust SDK tutorial).

If you are relying on network requesters then chances are that the IPs or domains your app relies on will not already be on the whitelist. Ideally, you would [run your own,](https://nymtech.net/operators/nodes/network-requester-setup.html) but we will also run a few nodes in ‘open proxy’ mode and share the addresses so that you can use them when beginning to develop. 

## Node Details:
- NR1
  - Location: Singapore
  - Nym Address: `FDeWfd8q686PWLXJDCqNJTCbydTk1KSux5HVftimsPyx.9XyThN4yh92eTMuLp1NvWicRZob8Ei5xpba9dvcMLxcN@9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J`
- NR2
  - Location: Frankfurt
  - Nym Address: `BNypKaGiGY8GNRN4gpV95GcaVS8n7CrHuoZNgQ2ezqv2.ACpaixzuaSzuMajVQj6aR7cbpbvp676tm21MiLbX1gni@678qVUJ21uwxZBhp3r56z7GRf6gMh3NYDHruTegPtgMf`