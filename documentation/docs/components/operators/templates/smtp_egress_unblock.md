Two variants are provided. Pick whichever fits your threat model and how much you want to disclose to your hoster.

### Variant A - minimal, no project disclosure

This is the wording that actually got OneProvider to lift the filter on three of my own servers. It does not mention Nym, mixnets, exit gateways or any specific use case beyond "I want to send mail." If you would rather not disclose to your hoster that you are running a Nym node, use this one.

```
Subject: Request to unblock outbound SMTP ports (25, 465, 587) on my servers

Hello,

I am operating the following dedicated servers with you:

  <IP_1> (<LOCATION_1>)
  <IP_2> (<LOCATION_2>)
  <IP_3> (<LOCATION_3>)

Could you please unblock outbound TCP on ports 25, 465 and 587 on all of them?

Currently the ports are blocked at the network edge on each of these servers.
SYN packets leave the server but no response is ever returned (confirmed with
tcpdump). The same ports work fine on another server I have with you in a
different region, so the block appears to be specific to these datacenters'
network policy.

All listed IPs are clean on every major blocklist (Spamhaus, SORBS,
Barracuda, SpamCop, UCEPROTECT, PSBL and others - verified today), so there
is no abuse history against any of them.

Thank you.
```

### Variant B - full disclosure of Nym context

If you are comfortable telling your hoster what the server actually does (some hosters appreciate the context, and it can help when arguing the case), use this version.

```
Subject: Request to unblock outbound SMTP ports (25, 465, 587)

Hi,

I am operating a dedicated server with you at <YOUR_SERVER_IP> running a "nym
node" - part of the Nym mixnet (https://nym.com/), a privacy-preserving
network similar in spirit to Tor.

I would like to request the unblocking of outbound TCP on ports 25, 465 and
587 on this server.

Currently these ports are filtered at your network edge: SYN packets leave
the server but no response is ever returned (confirmed via tcpdump from my
side). This prevents legitimate users of the Nym network from using their
mail clients (Thunderbird, Apple Mail, Outlook etc.) to relay outgoing email
through their normal SMTP submission server while connected via my exit
gateway.

The ports in question are documented in the official Nym exit policy at
https://nymtech.net/.wellknown/network-requester/exit-policy.txt

To address abuse concerns up front:

1. Nym exit gateways apply a per-source-IP rate limit on outbound SMTP at the
   firewall level (iptables `hashlimit` rule). A single user cannot use the
   node as a high-volume spam relay.
2. The IP address <YOUR_SERVER_IP> is currently clean on all major public
   blocklists (Spamhaus ZEN/SBL/XBL/PBL, Barracuda, SpamCop, SORBS,
   UCEPROTECT L1/L2/L3, PSBL, Truncate). I am happy to provide screenshots
   or reverify on request.
3. If abuse occurs, I am reachable at <YOUR_ABUSE_CONTACT> and will
   investigate any complaints promptly.

Could you please confirm whether the unblock can be applied and on what terms?

Thank you,

<YOUR_NAME/PSEUDONYM>
```
