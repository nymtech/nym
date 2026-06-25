# family-page-alignment Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Family page horizontal padding matches wallet standard
The Family page container SHALL use the same horizontal padding as other wallet pages (Balance, Bonding, Delegation) so that cards align to the standard content gutter.

#### Scenario: Cards reach standard horizontal extent
- **WHEN** the Family page is displayed
- **THEN** the NymCard components SHALL align horizontally with cards on the Balance and Bonding pages

#### Scenario: Vertical spacing is preserved
- **WHEN** the Family page is displayed
- **THEN** vertical spacing between sections SHALL be maintained (no regression to zero-gap layout)

