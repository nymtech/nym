# family-members-table Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Unified members table mirrors the Delegations tab (supersedes NYM-1560 "See all")

The owner management surface SHALL present every family node in a single delegations-style table with three columns — Node, Status, Actions — one row per record. This replaces the earlier `MemberList` + `PendingInvitesList` pair and the planned "See all (N)" truncation of the Removed/Rejected sections. History is kept clean by de-duplication (below) rather than by collapsing.

#### Scenario: One table for all member records
- **WHEN** the owner views their family
- **THEN** joined members, pending invites, rejected nodes, and removed nodes SHALL all appear as rows in one table with columns Node, Status, Actions
- **THEN** rows SHALL be ordered current members first, then outstanding invites, then historical (rejected, removed) records

#### Scenario: Status column reflects each row's state
- **WHEN** a row is rendered
- **THEN** its Status cell SHALL show a status chip for joined / pending / expired / rejected / removed
- **THEN** an active pending invite SHALL additionally show its live expiry countdown next to the chip

#### Scenario: Actions column shows only the applicable action
- **WHEN** a joined member row is rendered
- **THEN** the Actions cell SHALL offer a "Remove" action
- **WHEN** an active pending invite row is rendered
- **THEN** the Actions cell SHALL offer a "Withdraw" action
- **WHEN** an expired pending invite row is rendered
- **THEN** the Actions cell SHALL offer a "Clear" action
- **WHEN** a rejected or removed row is rendered
- **THEN** the Actions cell SHALL be empty (no action, and no placeholder dash)

### Requirement: A currently-joined node is not duplicated in history

A node that is currently in the Joined state SHALL NOT also appear in the Rejected or Removed rows, even if it was previously rejected or removed before re-joining. De-duplication happens when deriving the member sections.

#### Scenario: Re-joined node appears once
- **WHEN** a node was previously removed/rejected and is now joined
- **THEN** it SHALL appear only in the Joined rows and SHALL NOT appear in the Removed or Rejected rows

### Requirement: Table loading, empty, and error states

The members table SHALL communicate its data state without losing already-loaded rows on a transient refetch error.

#### Scenario: Loading state before first data
- **WHEN** the member list is loading and no rows are available yet
- **THEN** the table SHALL show a "Loading members…" indicator

#### Scenario: Empty state
- **WHEN** the family has no members or invites
- **THEN** the table SHALL show a "No members or invites yet." empty message

#### Scenario: Error state with retry
- **WHEN** the member list fails to load and there is no prior data to show
- **THEN** the table SHALL show a "Failed to load the member list." message with a Retry control
- **WHEN** a refetch fails but data was already loaded
- **THEN** the previously loaded rows SHALL remain visible (no error wipeout)

#### Scenario: Manual refresh
- **WHEN** the user clicks the table's Refresh control
- **THEN** the member list SHALL refetch

