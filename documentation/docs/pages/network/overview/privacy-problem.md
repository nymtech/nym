# The Privacy Problem

## Metadata is the message

When you communicate over the internet, you can think of two types of information being transmitted:
- The **content** is the actual message, file, or data being sent. In the context of a messaging app, this is the contents of your message. In the context of something lower level, like an HTTP packet, this is the encrypted payload of the packet itself.
- The **metadata** is information about the communication itself, some of which can be gathered immediately, such as HTTP packets have headers which show the sending and receiving IP addresses (revealing which devices are communicating), timestamps, packet sizes hinting at what type of content and what connection type (e.g. the different [Maximum Tranmission Units of different media](https://en.wikipedia.org/wiki/Maximum_transmission_unit#MTUs_for_common_media)), and some which is gathered over time, by finding patterns in large amounts of traffic, such as frequency patterns indicating how often parties interact.

Traditional encryption like TLS and end-to-end-encryption (E2EE) protect content - this is what is often the [focus of media attention](https://wire.com/en/blog/whatsapp-end-to-end-encryption-risks). However, most solutions either don't protect from metadata analsis, or falsely purport to do so.

This matters because metadata can reveal social graphs showing who knows whom, behavioral patterns exposing daily routines and habits, sensitive activities like medical consultations or legal advice, and location history tracking where you've been and when. As former NSA Director Michael Hayden put it: ["We kill people based on metadata."](https://committees.parliament.uk/writtenevidence/36962/html/)

## The adversary models

When using the **Mixnet mode** the Nym Network is designed to protect against **Global Passive Adversaries**—entities capable of observing traffic across the entire network simultaneously. This includes nation-state intelligence agencies, large corporations with extensive network infrastructure, ISPs, and collaborative adversaries sharing data.

The assumption is that these adversaries can monitor all entry and exit points, correlate timing across the network, apply machine learning to traffic patterns, and conduct long-term statistical analysis. When Tor was first deployed in 2002, such attacks were considered science fiction. They are now documented reality.

**dVPN mode** offers reduced protections against E2E surviellance and timing analsis, but still offers similar protections to Tor whilst offering increased speeds.

## Why traditional solutions fall short

**VPNs** provide a single point of trust. Most VPN providers see your traffic movements, can be legally or financially compelled to log, and your payment or account information (in most cases) links directly to your usage.

**Tor** was designed before global passive adversaries were considered realistic. It provides no timing obfuscation and no cover traffic, making it vulnerable to end-to-end correlation attacks.

## Nym's approach

The Nym Network addresses these limitations through decentralization (no single entity to trust or compromise), and anonymous credentials (ensuring payment cannot be linked to usage) in **dVPN mode**.

**Mixnet mode** adds packet mixing (reordering traffic to break timing correlation), cover traffic (generating dummy packets indistinguishable from real ones), and uniform packet sizes (preventing content-type inference),
