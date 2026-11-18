# Tasks

Owner of each section: TBD(Max). This change depends on `add-codex-snippets` (records, statements, coverage report, stage 7 generator) and on `add-mixnet-core-specs` (the first requirements that link to a threat-model property).

## 1. Join the rules and the records (A6)

- [ ] 1.1 Push the branch of the rules experiment, or name it here: TBD(Max). Compare it with `max/codex-snippets`, and list each file that both change
- [ ] 1.2 Merge the rules and the CI gate into the codex branch, or into `develop` first. Owner of the merge order: TBD(Max)
- [ ] 1.3 Run the threat-model adapter before and after. The output is identical apart from `warrant`. The model owner resolves each verdict that changes

## 2. Warrant (A1)

- [ ] 2.1 Set `meta.warrant` in the record builder, by the table in A1
- [ ] 2.2 Add the warrant counts for `normative` records to the coverage report
- [ ] 2.3 Tests: one record of each warrant, and the requirement that changes from `asserted` to `checked` when it gains a witness

## 3. Witnesses and canaries (A2)

- [ ] 3.1 Accept `meta.witness` and `meta.canary` on statement records
- [ ] 3.2 Refuse promotion to `enforced` with no witness, no canary, or no canary date
- [ ] 3.3 Add the two columns to the coverage report
- [ ] 3.4 Write a mutation runner that applies each canary to a copy of the source and confirms that the witness fails. Start with the statements of `add-mixnet-core-specs`
- [ ] 3.5 Record the measure: the share of `enforced` statements whose canary turned red in the last run

## 4. Obligations (A3)

- [ ] 4.1 Define the obligation record and the records-check rule for an empty owner
- [ ] 4.2 Set the period after which a `normative` record with no witness opens an obligation: TBD(Max) days
- [ ] 4.3 Write obligations from accepted `contradicts` verdicts, when the coherence agent exists
- [ ] 4.4 After four weeks, record the measure of A3, and apply its result

## 5. Assurance viewpoint (A4)

- [ ] 5.1 Review `viewpoints/assurance.json` and `viewpoints/prompts/assurance-summary.md`. Owner: TBD(Max)
- [ ] 5.2 Move them to the codex site when the stage 7 generator exists
- [ ] 5.3 Generate the view and record the stage 7 measure: the share that levels 0 and 1 make
- [ ] 5.4 Compare the view with one hand-made audit page, as stage 7 of `add-codex-snippets` asks

## 6. Sufficiency report (A5)

- [ ] 6.1 Add the record kinds and facets to each concern of the assurance viewpoint
- [ ] 6.2 Write the deterministic part of the report
- [ ] 6.3 A person marks each concern of the first view. Record who marked it
- [ ] 6.4 Record the measure, and apply the result of A5 if the two parts disagree

## 7. Label experiment (A7)

- [ ] 7.1 Write about 20 scenarios where the label matters. Keep them next to the stage 0 query sets
- [ ] 7.2 Add a switch to the MCP server that removes `authority` and `warrant` from responses
- [ ] 7.3 Run each scenario in both arms with one agent and one model. Needs an API key: TBD(Max)
- [ ] 7.4 A person reads the transcripts and records the measure
