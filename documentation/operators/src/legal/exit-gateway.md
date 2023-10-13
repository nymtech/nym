# Nym operators - Running Exit Gateway

```admonish info
The entire content of this page is under [Creative Commons Attribution 4.0 International Public License](https://creativecommons.org/licenses/by/4.0/).
```

This page is a part of Nym Community Legal Forum and its content is composed by shared advices in [Node Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) (Matrix chat) as well as though pull requests done by the node operators directly to our [repository](https://github.com/nymtech/nym/tree/develop/documentation/operators/src), reviewed by Nym DevRels.

This document presents an initiative to further support Nym’s mission of allowing privacy for everyone everywhere. This would be achieved with the support of Nym node operators operating gateways and opening these to any online service with the safeguards of the [Tor Null ‘deny’ list](https://tornull.org/).


```admonish warning
Nym core team cannot provide comprehensive legal advice across all jurisdictions. Knowledge and experience with the legalities are being built up with the help of our counsel and with you, the community of Nym node operators. We encourage Nym node operators to join the operator channels ([Element](https://matrix.to/#/#operators:nymtech.chat), [Discord](https://discord.com/invite/nym), [Telegram](https://t.me/nymchan_help_chat)) to share best practices and experiences.
```


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

As of now we the gateways will be defaulted to Tornull’s (note: Not affiliated with Tor) deny list - reproduction permitted under Creative Commons Attribution 3.0 United States License which is IP-based, e.g., `ExitPolicy reject 5.188.10.0/23:*`. Whether we will stick with this list, do modifications (likely) or compile another one is still a subject of discussion.

<:--
These policies will be either reused without modification from Tor / Tornull (license permitting), or customised and updated in a Nym crowdsourced community effort.
-->

The Gateways will display an HTML page similar to that suggested by [Tor](https://gitlab.torproject.org/tpo/core/tor/-/raw/HEAD/contrib/operator-tools/tor-exit-notice.html) for exit relays on port 80 and port 443. This will allow the operator to provide information about their Gateway, possibly including the currently configured exit policy, without having to actively communicate with law enforcement or regulatory authorities. It also makes the behaviour of the Gateway transparent and even computable (a possible feature would be to offer a machine readable form of the notice in JSON or YAML).

We also recommend operators to check the technical advice from [Tor](https://community.torproject.org/relay/setup/exit/).

## Legal environment - Findings from our legal team

The Swiss legal counsel and US legal counsel have so far provided the following advice:

### Switzerland

TBD soon.

### United States

A US counsel shared the following advice:

The legal risk faced by VPN operators subject to United States jurisdiction depends on various statutes and regulations related to privacy, anonymity, and electronic communications. The key areas to consider are: intermediary liability and exceptions, data protection, copyright infringement, export controls, criminal law, government requests for data and assistance, and third party liability.

As outlined in Part A, the United States treats VPNs as telecommunications networks subject to intermediary liability protection from wrongful conduct that occurs on its network. However, such protections do have exceptions including criminal law and copyright claims that are worth considering. In the United States, I am not aware of an individual ever being prosecuted or convicted for running a node for a dVPN or a Privacy Enhancing Network.

However, as discussed in Part B-C, VPN operators are subject to law enforcement requests for access or assistance in obtaining access to data relevant to an investigation into allegedly unlawful conduct that was facilitated by the network as an intermediary. As shown in Part C, governments may also request assistance from node operators for certain high-level and national security targets.

Finally, as outlined in Parts D-G, VPN operators may also be subject to non-criminal liability including (Part D) failing to respond to notices under the DMCA, (Part E) privacy and data protection law, (Part F) third party lawsuits stemming from wrongful acts committed using the network, and (G) export control violations.
