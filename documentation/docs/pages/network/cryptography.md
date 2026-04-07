---
title: "Nym Network Cryptography"
description: "Overview of the cryptographic systems powering Nym: transport encryption, Sphinx packet format, per-hop encryption, and zk-nym anonymous credentials."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Cryptography

The Nym Network relies on several cryptographic systems working together. This section covers the algorithms, packet formats, and credential systems that provide privacy guarantees.

## What's covered

[Sphinx Packets](/network/cryptography/sphinx) explains the packet format that enables layered encryption and anonymous routing. Each Sphinx packet contains routing information encrypted in layers, where each hop can only decrypt its own layer.

[zk-nyms](/network/cryptography/zk-nym) covers the anonymous credential system that separates payment from usage. This is how you can pay for network access without that payment being linkable to your activity.
