# Nym operators - Running Exit Gateway

```admonish info
The entire content of this page is under [Creative Commons Attribution 4.0 International Public License](https://creativecommons.org/licenses/by/4.0/).
```

This page is a part of Nym Community Legal Forum and its content is composed by shared advices in [Node Operators Matrix channel](https://matrix.to/#/#operators:nymtech.chat) as well as though direct pull requests done by the node operators directly to our [repository](https://github.com/nymtech/nym/tree/develop/documentation/operators/src).

This document presents an initiative to further support Nym’s mission of allowing privacy for everyone everywhere. This would be achieved with the support of Nym node operators operating gateways and opening these to any online service with the safeguards of the [Tor Null ‘deny’ list](https://tornull.org/).

## Summary

* This document outlines a plan to change Nym Gateways from operating with an ‘allow’ to a ‘deny’ list to enable broader uptake and usage of the Nym mixnet. It provides operators with an overview of the plan, pros and cons, legal as well as technical advice. 

* Nym is committed to ensuring privacy for all users, regardless of their location and for the broadest possible range of online services. In order to achieve this aim, the Nym mixnet needs to increase its usability across a broad range of apps and services.

* Currently, Nym Gateway nodes only enable access to apps and services that are on an ‘allow’ list that is maintained by the core team. 

* To decentralise and enable privacy for a broader range of services, this initiative will have to transition from the current ‘allow’ list to a ‘deny’ list (based on the [Tor Null advisory BL](https://tornull.org/)). 

* This will enhance the usage and appeal of Nym products for end users. As a result, increased usage will ultimately lead to higher revenues for Nym operators.

* Nym core team cannot provide operators with definitive answers regarding the potential risks of operating open Gateways. However, there is online evidence of operating Tor exit relays:
	* From a technical perspective, Nym node operators may need to implement additional controls, such as dedicated hardware and IP usage, or setting up an HTML exit notice on port 80.
	* From an operational standpoint, node operators may be expected to actively manage their relationship with their ISP or VPS provider and respond to abuse requests using the proposed templates.
	* Legally, exit relays are typically considered "telecommunication networks" and are subject to intermediary liability protection. However, there may be exceptions, particularly in cases involving criminal law and copyright claims. Operators could seek advice from local privacy associations and may consider running nodes under an entity rather than as individuals.

* This document serves as the basis for a consultation with Nym node operators on any concerns or additional support and information you need for this change to be successful and ensure maximum availability, usability and adoption.

## Goal of the initiative

**Nym supports privacy for everyone, everywhere.**

To offer a better and more private everyday experience for its users, Nym would like them to use any online services they please, without limiting its access to a few messaging apps or crypto wallets.

To achieve this, operators running “gateways” would have to “open” their nodes to a wider range of online services, in a similar fashion to Tor exit relays.

## Pros and cons of the initiative

Previous setup: Running nodes supporting strict SOCKS5 app-based traffic

| **Dimension** | **Pros** | **Cons** |
| :--- | :--- | :--- |
| Aspirational |   | - Very limited use cases, not supportive of the “Privacy for everyone everywhere” aspiration<br>- Limited appeal to users, low competitiveness in the market, thus low usage |
| Technical | - No changes required in technical setup |   |
| Operational | - No impact on operators operations (e.g., relationships with VPS providers)<br>- Low overhead<br>- Can be run as an individual |   |
| Legal | - Limited legal risks for operators |    |
| Financial |    | - Low revenues for operators due to limited product traction | 


The new setup: Running nodes supporting traffic of any online service (with safeguards in the form of an denylist)

| **Dimension** | **Pros** | **Cons** |
| :--- | :--- | :--- |
| Aspirational | - Higher market appeal of a fully-fledged product able to answer all users’ use cases<br>- Relevance in the market, driving higher usage |   |
| Technical | - Very limited changes required in the technical setup (changes in the allow -> denylist) | - Increased monitoring required to detect and prevent abuse (e.g. spam) |
| Operational |    | - Higher operational overhead, such as dealing with DMCA / abuse complaints, managing the VPS provider questions, or helping the community to maintain the denylist <br>- Administrative overhead if running nodes as a company or an entity |
| Legal |   | - Ideally requires to check legal environment with local privacy association or lawyer | Financial | - Higher revenue potential for operators due to the increase in network usage | - If not running VPS with an unlimited bandwidth plan, higher costs due to higher network usage |

## New gateway setup

In our previous technical setup, network requesters acted as a proxy, and only made requests that match an allow list. That was a default IP based list of allowed domains stored at Nym page in a centralised fashion possibly re-defined by any Network requester operator. 

This restricts the hosts that the NymConnect app can connect to and has the effect of selectively supporting messaging services (e.g. Telegram, Matrix) or crypto wallets (e.g. Electrum or Monero). Operators of network requesters can have confidence that the infrastructure they run only connects to a limited set of public internet hosts.

In the new setup, the main change is to expand this short allow list to a more permissive setup. An exit policy will constrain the hosts that the users of the Nym Mixnet and Nym VPN can connect to. This will be done in an effort to protect the operators, as Gateways will act both as SOCKS5 Network Requesters, and exit nodes for IP traffic from Nym Mixnet VPN and VPN clients (both wrapped in the same app). 





