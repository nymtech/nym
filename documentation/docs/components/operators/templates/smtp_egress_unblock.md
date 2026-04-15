```
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
   UCEPROTECT L1/L2/L3, PSBL, Truncate). I am happy to provide screenshots or
   reverify on request.
3. If abuse occurs, I am reachable at <YOUR_ABUSE_CONTACT> and will
   investigate any complaints promptly.

Could you please confirm whether the unblock can be applied and on what terms?

Thank you,

<YOUR_NAME/PSEUDONYM>
```
