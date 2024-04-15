# VPS Setup

## VPS Hardware Specs

You will need to rent a VPS to run your node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol).

Currently we run [performance testing](../testing/performance.md) events to find out the best optimization. Sphinx packet decryption is CPU-bound, so more fast cores the better throughput.

Before we conclude the testing, these are the rough specs:

| **Hardware** | **Minimum** |
| :---         | ---:        |
| CPU Cores    | 2           |
| Memory       | 4 GB RAM    |
| Storage      | 40 GB       |
| Bandwidth    |             |

## VPS Configuration

<!--
Add here:
- IPv4 and IPv6 setup
- Firewall and port configuration
- Links to Gateway specific configuration of VPS, WSS, proxy
-->
