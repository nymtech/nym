# nymcard-testid-contract Specification

## Purpose
TBD - created by archiving change fix-nymcard-data-testid. Update Purpose after archive.
## Requirements
### Requirement: NymCard exposes a DOM test id from a standard prop

`NymCard` SHALL apply a caller-provided test id to its root DOM element. It MUST accept the standard `data-testid` prop, and SHALL continue to accept the legacy `dataTestid` prop; when both are provided, one resolved value is used. The resolved test id MUST appear on exactly one element (the card root), never duplicated across the root and the header.

#### Scenario: data-testid reaches the DOM

- **WHEN** a `NymCard` is rendered with `data-testid="member-list"`
- **THEN** the rendered card root element has `data-testid="member-list"`
- **AND** a scoped query within that element can find the card's content

#### Scenario: legacy dataTestid still works

- **WHEN** a `NymCard` is rendered with `dataTestid="foo"` and no `data-testid`
- **THEN** the rendered card root element has `data-testid="foo"`

#### Scenario: no duplicate ids

- **WHEN** a `NymCard` with a header and a provided test id is rendered
- **THEN** exactly one element carries that test id (strict locators match a single node)

### Requirement: No title-derived test ids

`NymCard` SHALL NOT derive a test id from its `title` (or emit a `nym-card` fallback). When no test id is provided, the card SHALL render without a `data-testid` attribute.

#### Scenario: untagged card emits no test id

- **WHEN** a `NymCard` is rendered with a `title` but no `data-testid`/`dataTestid`
- **THEN** neither the card root nor its header carries a `data-testid` derived from the title

