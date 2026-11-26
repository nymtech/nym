# Specifications

This directory holds reference specifications for parts of the Nym network, derived from the code in this repository. Code references are given as `path` (`symbol`) so they survive line drift. Where the code and a specification disagree, the code is authoritative.

## Services

- [Node Status API](services/node-status-api/README.md): the aggregation and read service for node directory state, geolocation, packet statistics and gateway probe results. It also schedules gateway test runs, which `nym-node-status-agent` workers claim and run, and funds them from its own ecash ticketbook buffer.

## Tools

- [Gateway Probe](tools/gateway-probe/README.md): `nym-gateway-probe`, the binary and library that test whether a gateway's advertised transports work.

## Technical details

- [Node Status API](technical-details/node-status-api/README.md): design, monitor cycle, HTTP surface, persistence, test runs, ticketbook issuance and geodata.
- [Gateway Probe](technical-details/gateway-probe/README.md): design, probe tests, result shape and consumers.
