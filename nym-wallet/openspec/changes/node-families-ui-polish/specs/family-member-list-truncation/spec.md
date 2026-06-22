## ADDED Requirements

### Requirement: Removed and Rejected sections are collapsed by default (NYM-1560)
The system SHALL limit the Removed and Rejected member sections to 3 visible entries by default and SHALL provide a "See all (N)" control to reveal the full list. History SHALL never be auto-deleted or silently hidden.

#### Scenario: More than 3 removed nodes — collapsed by default
- **WHEN** the Removed section contains more than 3 entries
- **THEN** only the first 3 entries SHALL be shown by default
- **THEN** a "See all ({total})" button SHALL be visible below the truncated list

#### Scenario: 3 or fewer removed nodes — no collapse control
- **WHEN** the Removed section contains 3 or fewer entries
- **THEN** all entries SHALL be shown
- **THEN** no "See all" button SHALL appear

#### Scenario: Expand reveals all entries
- **WHEN** the user clicks "See all (N)"
- **THEN** all entries in that section SHALL become visible
- **THEN** the expand button SHALL change to "Show less" (or equivalent collapse control)

#### Scenario: Same behaviour for Rejected section
- **WHEN** the Rejected section contains more than 3 entries
- **THEN** the same truncation and expand behaviour SHALL apply

#### Scenario: Pending and Joined sections are not truncated
- **WHEN** the Pending or Joined section contains any number of entries
- **THEN** all entries SHALL be shown without truncation
