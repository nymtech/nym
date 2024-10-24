# Anonymous Replies with SURBs

> SURBs are the Sphinx equivalent of "onion addresses" in Tor, with the caveat that a SURB can only
be used once (to prevent replay attacks) and within its epoch of validity (the mix node public keys used to
prepare the SURB are only valid for a limited period).
> ...
> A SURB effectively contains: (1) the encrypted headers of a Sphinx message that, if sent to the mixnet, will be routed back to the original sender; (2) the address of the first-layer mix node where the message should be sent; and (3) a cryptographic key to encrypt the reply payload.
>
> [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf) §4.6

As outlined in the [concepts](../concepts/anonymous-replies) section, SURBs are layer encrypted sets of Sphinx headers detailing a reply path ending in the sending client's [Nym address](../traffic/addressing-system). Clients receiving messages with SURBs attached are able to write a payload to the provided headers without ever learning about anything other than the first hop back into the Mixnet - the Gateway they are currently connected to.

There is a balance to be struck between the amount of SURBs to compute to send along with messages (which takes computation resources) and not sending enough SURBs initially, thus having to wait for a SURB to be sent from the receiver to the sender, requesting more SURBs be sent.
