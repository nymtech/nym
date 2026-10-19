## ADDED Requirements

### Requirement: Every metric is a labelled family

The registry SHALL represent every metric as a label family declared with an ordered list of label names, which MAY be empty. A family declared with no label names SHALL be exposed identically to a scalar metric of the same type, with no label set rendered.

#### Scenario: A family with no label names renders as a scalar

- **WHEN** a counter is registered with an empty list of label names, incremented, and the registry is encoded
- **THEN** the rendered exposition text is byte-identical to that of a plain `IntCounter` of the same name, help and value, carrying no `{}` label set

#### Scenario: A family with label names renders one series per combination

- **WHEN** a counter family declared with label name `kind` is incremented for two different label values
- **THEN** the rendered exposition text contains one `TYPE` line for the family and two series lines, each carrying its own label value and its own count

### Requirement: Metrics are registered once and addressed by their family name

The registry SHALL key each family by its fully qualified name, derived from the family's descriptor, and SHALL ignore a registration for a name it already holds. Metric names SHALL be sanitised so that the first character matches `[a-zA-Z_:]` and every subsequent character matches `[a-zA-Z0-9_:]`.

#### Scenario: Re-registering an existing name is a no-op

- **WHEN** a metric is registered under a name the registry already holds
- **THEN** the existing family is retained, the new one is discarded, and no error is returned to the caller

#### Scenario: Invalid name characters are replaced

- **WHEN** a metric is registered with a name containing characters outside the permitted set
- **THEN** each offending character is replaced with an underscore before registration

### Requirement: Recording resolves a child series from label values

Every recording operation SHALL accept label values and SHALL apply its effect only to the child series those values identify. Children under one family SHALL be independent of one another.

#### Scenario: Recording affects only the addressed child

- **WHEN** a gauge family declared with one label name is set to different values for two different label values
- **THEN** each child reports the value most recently set for its own label value, and neither reflects the other's

#### Scenario: A timer is started against a child

- **WHEN** a timer is started on a histogram family for a given set of label values and then observed
- **THEN** the resulting observation is recorded against that child alone

### Requirement: Unlabelled recording is the zero-label case

The registry SHALL retain recording methods that take no label values, with their existing signatures, and these SHALL behave exactly as the labelled form invoked with an empty list of label values. There SHALL be a single recording implementation, with the unlabelled methods delegating to it.

#### Scenario: Existing consumer code is unaffected

- **WHEN** a consumer registers and records a metric using only the methods that existed before labels were introduced
- **THEN** the metric is registered, the value is recorded, and the exposed output is unchanged from before

### Requirement: Misuse is reported and discarded, never fatal

The registry SHALL NOT panic on any recording operation. Where the supplied label values do not match the family's declared label-name arity, or where the operation does not apply to the family's type, the registry SHALL log a warning naming the family and SHALL leave every series unchanged.

#### Scenario: Label arity does not match

- **WHEN** a recording operation supplies a number of label values that differs from the family's declared label-name count
- **THEN** a warning naming the family is logged, no series is created or modified, and the call returns without panicking

#### Scenario: Operation does not apply to the metric type

- **WHEN** a set operation is applied to a counter family, or an observation is applied to a gauge family
- **THEN** a warning naming the family is logged and no series is modified

#### Scenario: Recording against an unregistered name

- **WHEN** a recording operation names a family the registry does not hold
- **THEN** the call reports that it did not apply, without panicking, so a caller can choose to register and retry

### Requirement: A child series can be created without recording a value

The registry SHALL expose an operation that creates a child series for a given set of label values so that the child is exposed from the next scrape, without recording an observation or altering a value.

#### Scenario: A histogram child is exposed before any observation

- **WHEN** a child of a histogram family is created through this operation and the registry is encoded
- **THEN** the child appears in the exposition with a sample count of zero and no bucket populated, so no synthetic observation has been introduced

#### Scenario: A counter child is exposed at zero

- **WHEN** a child of a counter family is created through this operation and the registry is encoded
- **THEN** the child appears in the exposition with a value of zero, so a consumer pre-creating its label space does not expose a first real observation as a counter reset

### Requirement: A child series can be removed

The registry SHALL expose an operation that removes a single child series identified by its label values, after which that child SHALL NOT appear in the exposition until it is recorded against or created again. The registry imposes no bound on the number of children a family may hold, so a consumer whose label values churn is responsible for removing children it will not record against again.

#### Scenario: A removed child stops being exposed

- **WHEN** a child of a family is recorded against, then removed, and the registry is encoded
- **THEN** that child's series is absent from the exposition while the family's other children remain

#### Scenario: Removing a child that does not exist

- **WHEN** a removal names label values for which no child exists
- **THEN** a warning is logged, every existing series is left unchanged, and the call returns without panicking
