# Gateways

> The gateway setup and maintenance guide has migrated to the [Operator Guides book](https://nymtech.net/operators/nodes/gateway-setup.html).

REWRITE AND INCLUDE ALL THE STUFF ABOUT INGRESS AND EXIT GATEWAYS

Gateways are key to both the usability of the mixnet, and the operation of the mixnet's tokenomics. They serve two main functions:
* In the future (when the mixnet is no longer running in 'free to use' mode), to check for zkNym credentials (previously referred to as [Coconut Credentials](../bandwidth-credentials.md)) with which users can anonymously prove they have paid to send traffic through the mixnet. A % of the worth of these credentials will be distributed to the operator of the gateway periodically as payment for providing their service. The more credentials user clients 'spend' with them, the higher the rewards for the gateway operator and their delegators will be. The rest of this value will be sent to the Mixmining Pool, a pool of tokens from which `NYM` rewards are distributed to mix node operators.
* Act as a mailbox for connected clients. Clients create a lasting relationship with a gateway on initialisation, binding themselves to always use a particular gateway as their ingress point for mixnet traffic (this also means that this gateway is the egress point for any traffic sent _to_ this client - see the [mixnet traffic flow page](../architecture/traffic-flow.md) and the [addressing scheme](../architecture/addressing-system.md) for further details). If a client is offline and the Gateway can't deliver packets addressed to it, they will hold these packets until the client comes back online.

## Further Reading
* [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf) section 4.2
* [Nym Blog: Gateways to Privacy](https://blog.nymtech.net/gateways-to-privacy-51196005bf5)
