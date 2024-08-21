# What are zkNyms?

zkNyms are an implementation of the [Coconut Credential scheme](./coconut.md). The linked page contains a deeper outline of the attributes granted by the scheme for those interested.

As outlined in the [overview](./zknym-overview.md) on the next page, zkNyms allow for users to pay for Mixnet access using non-anonymous cryptocurrencies or fiat but access the Mixnet (either via the NymVPN app or via applications with integrated Mixnet access via an SDK) in a manner that is **unlinkable to their payment account**; they are unlinkable, rerandomisable anonymous access credentials that are 'spent' with Gateways in order to anonymously prove that someone has paid for Mixnet access. This solves one of the fundamental privacy problems with the majority of VPNs and dVPNs in production today: the linkability of a user's session with their payment information, which can in the majority of cases be easily used to deanonymise them, either at the behest of an authority or by the service operators themselves.

The current zkNym scheme is non-generic in that it is only used for gating Mixnet access. A generic 'Offline Ecash' scheme based on zkNyms is being actively researched and developed, in order to facilitate more generic and customisable anonymous credentials for other applications and services.
