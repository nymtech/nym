# smol-core-stack Spec Delta

## ADDED Requirements

### Requirement: Entry-gateway rotation on repeated common-mode failure

When auto-discovery exit rotation fails against a bounded number of distinct exits on
the current entry gateway (default 3), tunnel establishment SHALL treat the failure as
likely common-mode (the entry gateway or the send path through it, not the exits) and
SHALL escalate by re-registering the base client with a different entry gateway, then
retrying exit discovery and rotation on the new entry. Because entry-gateway
registration is sticky (it derives a per-client shared key), the escalation SHALL
re-register rather than repoint a live connection. Entry rotation SHALL be bounded by a
maximum number of entry gateways (default 3) so a network-wide outage terminates with a
clear error rather than cycling entries indefinitely. A rotated entry SHALL exclude the
entry gateway that just failed.

#### Scenario: Common-mode exit failure escalates to a new entry

- **WHEN** every exit attempt against the current entry gateway fails its handshake with
  no response, up to the exit-attempt bound
- **THEN** establishment re-registers with a different entry gateway and retries exit
  selection on it, rather than continuing to rotate exits on the failed entry

#### Scenario: A healthy entry recovers the tunnel

- **WHEN** the current entry gateway is the fault and a different entry gateway is
  healthy
- **THEN** re-registering with the healthy entry and retrying exit selection establishes
  the tunnel

#### Scenario: Bounded escalation terminates on a total outage

- **WHEN** exit rotation and entry rotation both fail across their bounds
- **THEN** establishment fails with a clear error rather than cycling entry gateways
  indefinitely

#### Scenario: Established tunnels are not repointed

- **WHEN** a tunnel is already established and later degrades
- **THEN** entry rotation does not apply; it is an establishment-time recovery only
