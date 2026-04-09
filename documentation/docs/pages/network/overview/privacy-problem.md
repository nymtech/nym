---
title: "The Privacy Problem: Why Metadata Matters"
description: "Why metadata exposure is a critical privacy threat, how adversaries exploit traffic patterns, and why traditional solutions like VPNs, Tor, and E2EE fall short."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# The Privacy Problem

## Metadata

When you communicate over the internet, two types of information are in play:
- The **content** is the actual message, file, or data being sent.
- The **metadata** is everything else: who is talking to whom, when, from where, and how often. Some metadata is visible in every packet (source/destination IPs, timestamps, sizes), wheras other metadata only emerges from patterns over time: interaction frequency, session durations, and behavioural fingerprints that can identify users across sessions. See [Maximum Transmission Units](https://en.wikipedia.org/wiki/Maximum_transmission_unit#MTUs_for_common_media) for one example of what packet sizes reveal.

TLS and end-to-end encryption protect content, which is often the [focus of media attention](https://wire.com/en/blog/whatsapp-end-to-end-encryption-risks). However, most solutions don't protect metadata at all, and some falsely claim to.

Metadata alone is enough to reconstruct who you talk to, when, and from where. Intelligence agencies know this; as former NSA Director Michael Hayden put it, ["We kill people based on metadata."](https://committees.parliament.uk/writtenevidence/36962/html/)

## The adversary models

**Mixnet mode** is designed to protect against **Global Passive Adversaries**: entities that can observe traffic across the entire network at once, such as nation-state level agencies, large corporations with broad network infrastructure, ISPs, or any combination sharing data.

The assumption is worst-case: the adversary monitors all entry and exit points, correlates timing, applies machine learning to traffic patterns, and runs long-term statistical analysis. When Tor launched in 2002, this was considered unrealistic - machine learning and the increase in computation power have made this unfortunately more of a potential reality today.

**dVPN mode** does not defend against timing analysis, but it splits trust across two independent operators and removes payment linkability, which already addresses the biggest weaknesses of traditional VPNs.

For a comparison with VPNs, Tor, and I2P, see [Nym vs Other Systems](/network/overview/comparisons). For help picking a mode, see [Choosing a Mode](/network/overview/choosing-a-mode).
