# The Privacy Problem

## Metadata is the message

When you communicate over the internet, two types of information are transmitted. The **content** is the actual message, file, or data being sent. The **metadata** is information about the communication itself—IP addresses revealing who is communicating, timestamps showing when, packet sizes hinting at what type of content, and frequency patterns indicating how often parties interact.

Traditional encryption like TLS and end-to-end encryption protects content well. Metadata remains exposed.

This matters because metadata can reveal social graphs showing who knows whom, behavioral patterns exposing daily routines and habits, sensitive activities like medical consultations or legal advice, and location history tracking where you've been and when. As former NSA Director Michael Hayden put it: "We kill people based on metadata."

## The adversary model

The Nym Network is designed to protect against **Global Passive Adversaries**—entities capable of observing traffic across the entire network simultaneously. This includes nation-state intelligence agencies, large corporations with extensive network infrastructure, ISPs, and collaborative adversaries sharing data.

These adversaries can monitor all entry and exit points, correlate timing across the network, apply machine learning to traffic patterns, and conduct long-term statistical analysis. When Tor was first deployed in 2002, such attacks were considered science fiction. They are now documented reality.

## Why traditional solutions fall short

**VPNs** provide a single point of trust. The VPN provider sees all your traffic, can be compelled to log, and your payment links directly to your usage. There's no timing protection—traffic patterns flow through unchanged.

**Tor** was designed before global passive adversaries were considered realistic. It provides no timing obfuscation and no cover traffic, making it vulnerable to end-to-end correlation attacks. Its centralized directory authority is another trust point.

**End-to-end encryption** like Signal protects content excellently but leaves metadata fully exposed. The server still sees who communicates with whom and when.

## Nym's approach

The Nym Network addresses these limitations through decentralization (no single entity to trust or compromise), packet mixing (reordering traffic to break timing correlation), cover traffic (generating dummy packets indistinguishable from real ones), uniform packet sizes (preventing content-type inference), and anonymous credentials (ensuring payment cannot be linked to usage).

The result is a network where observers—even those watching everything—cannot determine who is communicating with whom or when real communication is occurring.
