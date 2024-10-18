# Double Spend Protection
Double spend protection in the context of zk-nym is a balancing act between speed, reliability, and UX. There are two possible modes for protecting against attempted double spending of zk-nyms:

- Online: The online approach mandates that ingress Gateways instantly deposit zk-nyms received from clients to the NymAPI Quorum for verification. Once verified by the Quroum, the ingress Gateway is paid proprtional to the amount of bandwidth 'spent' with the zk-nym, and proceeds to grant the client access to the network.
- Offline: In contrast, the offline approach involves the periodic submission of collected zk-nyms by ingress Gateways to the Quorum, instead of an instant check. Subsequently, the Quorum nodes perform checks to detect any instances of double-spending and identify the public key associated with such occurrences, whereas the ingress Gateways only do a simple check to check that _that particular_ zk-nym had not been spent with itself before.

> The zk-nym system takes the **offline** approach.

## Offline Approach: Pros & Cons
The advantages of the offline approach are manifold:
- Immediate access to the Nym network upon zk-nym submission, eliminating any delays in service provisioning until payments are deposited and verified as would occur in the online approach.
- Alleviates performance strain on ingress Gateways and Quorum members, serving as a more efficient method compared to the online counterpart. By moving computationally intense work to the Quorum, this means that Gateway nodes are able to be run on less powerful machines, meaning more operators can more easily run them (and cover their costs) and thus increase the overall number and spread of Gateways around the globe.
- Moreover, the offline approach can circumvent the potential issue of overwhelming the blockchain with the serial numbers of spent coins.

However, the offline approach introduces certain limitations.
- Ingress Gateways accept zk-nyms without preemptively checking for instances of double spending thus making them susceptible to unknowingly accepting double-spent credentials.
- Any potential repercussions against double spenders can only be implemented once the user requests a new credential for their zk-nym Generator (aka they have to 'top up' and buy more bandwidth allowance), assuming they haven't altered their identifier (the Bech32 address).

An exploitable scenario arises from these limitations:
- A malicious user purchases bandwidth and aggregates a valid zk-nym credential in the standard way, worth $10 of crypto/fiat. Subsequently, the malicious user proceeds to sell the credential to 100 users for $1 each, allowing each user to generate zk-nym credits of 100MB from this **valid** credential. Under the offline approach, entry nodes forego double-spending checks; so long as the clients all used different ingress Gateways, all 100 users could access the network without obtaining a subscription. As bandwidth consumption is tracked locally between client and ingress node, and each zk-nym credit is rerandomised, there is no way that ingress Gateways would know that the zk-credential used by the client has been shared with other parties. This loophole highlights the need for stringent measures to counter such potential abuses within the system, without creating either speed bottlenecks (in the case of the Online model) or impacting the anonymity of the system. We can, however, mitigate this problem without doing either of these things.

## Solution to Offline Double Spending
To efficiently prevent the fraudulent use of tickets within the Nym network, a two-tiered solution is in place that combines (1) the immediate detection of double-spending attempts at the level of individuals ingress Gateways and (2) subsequent identification and blacklisting of offending clients at the Quorum level.

### Entry Node Implementation: Real-Time Ticket Unspending Validation
Each spent zk-nym credit contains as an attribute a unique serial number, which is revealed in plaintext to the respective ingress Gateway. Each Gateway has a copy of a [Bloom Filter](https://www.geeksforgeeks.org/bloom-filters-introduction-and-python-implementation/) - on receiving a credit, it will check against its copy of a local database to check whether this serial number has already been seen. If so, it rejects the credit as being double-spent and the client's connection request is rejected. If not, it will add the serial number to its local DB.

> Since each time a zk-nym credential is rerandomised its serial number is changed, the serial number being shared in no way identifies a client or user.

Each Gateway will periodically share their serial numbers with the Quorum and refresh their copy of the Bloom Filters from the Quorum, in order to refresh the global list shared by all ingress Gateways and the Quorum. See the step below for more on this.

> Crucially, ingress Gateways refrain from extensive computations to identify the original ticket owner, and avoids broadcasting information about the double-spending attempt to other ingress Gateways. The entry node is also not involved in any global blacklisting process of the clients. The sole purpose of this check is to swiftly identify any attempts at double-spending and add the seen ticket's serial number to the local DB cache.

### Nym-API Implementation: Blacklisting and Penalties for Double-Spenders
All Gateways periodically forward the collected credits to the Quorum, enabling them to pinpoint and blacklist any clients who double spend. Upon receiving the credits, the Quorum appends all the incoming serial numbers to the global list of spend zk-nym serial numbers and proceed with the identification process for any malicious users engaging in double-spending.

This identification phase involves looking for instances of double spending, identifying the id of the double-spending client, and blacklisting this client by its id. Subsequently, when this client requests a new credential, their plaintext public identifier is included in the request. The Quorum then checks if this identifier is blacklisted. If it is, a new credential is not issued. Furthermore, since the PSCs are only attainable after depositing NYM as payment, the Quorum has the authority to withhold the deposited NYMs as a punitive measure for any detected instances of double-spending.

<!-- extract parts of large diagram & upload as PNG to server + link -->
