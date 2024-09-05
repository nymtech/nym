# Network Requester

> The network requester setup and maintenance guide has moved to the [Operator Guides book](https://nymtech.net/operators/nodes/network-requester-setup.html).

Network requesters are the first instance of the catch-all term 'service', or 'service providers'. In essence, think of services as being the part of the mixnet infrastructure that let you _do_ something, such as access emails, messaging service backends, or blockchains via the mixnet. 

## Domain filtering
Network requesters, in essence, act as a form of proxy (somewhat analagous to a Tor exit node). If you have access to a server, you can run the network requester, which allows Nym users to send outbound requests from their local machine through the mixnet to a server, which then makes the request on their behalf, shielding them (and their metadata) from clearnet, untrusted and unknown infrastructure, such as email or message client servers.

By default the network requester is **not** an open proxy (although it can be used as one). It uses a whitelist for outbound requests.

Any request to a URL which is not on this local list (modified by the node operator) or [Nym's default list](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) will be blocked.

This default whitelist is useful for knowing that the majority of Network requesters are able to support certain apps 'out of the box', and the local whitelist allows operators to include their own whitelisted domains. 

> Substantial changes are on the horizon concerning how Network Requesters manage incoming requests - if you are an operator and have experience running software such as Tor exit nodes or p2p nodes get in touch via our [Matrix server](https://matrix.to/#/#dev:nymtech.chat). 

## (Coming soon) Consuming credentials for anonymous service payment 

## Further reading
* [Nym Blog: Network Requester deepdive](https://blog.nymtech.net/tech-deepdive-network-requesters-e5359a6cc31c)
* [Nym Blog: Choose Your Character](https://blog.nymtech.net/choose-your-character-an-overview-of-nym-network-actors-19e6a9808540)
