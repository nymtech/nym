# Gateways

> The gateway setup and maintenance guide has migrated to the [Operator Guides book](https://nymtech.net/operators/nodes/gateway-setup.html).

Gateways are key to both the usability of the mixnet, and the operation of the mixnet's tokenomics. They serve two main functions: 
* (When the mixnet is no longer in 'free' mode) to act literally as gateways; checking for zkNym credentials (previously referred to as [Coconut Credentials](../bandwidth-credentials.md)) that prove a user has paid to send traffic through the mixnet. A % of the worth of these credentials will be distributed to the operator of the gateway periodically as payment for providing their service. The more credentials user clients 'spend' with them (because of their quality of service) the higher the rewards. The rest of this value will be sent to the Mixmining Pool, a pool of tokens from which `NYM` rewards are distributed to mix node operators. 
* Act as a mailbox for connected clients. Clients create a lasting relationship with a gateway on initialisation, binding themselves to always use a particular gateway as their egress point for mixnet traffic, and always receive mixnet traffic from it (see the [mixnet traffic flow page](../architecture/traffic-flow.md) for further details). If a client is offline and the Gateway can't deliver packets addressed to it, they will hold these packets until the client comes back online. 

## Further Reading 
* TODO whitepaper section 
* TODO loopix paper section
* TODO blogpost 
