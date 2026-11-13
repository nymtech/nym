## Context

`add-codex-snippets` builds the records, the edges, the statements and the stage plan. This change adds four uses of the records and one join of two branches. Each decision below states its measure and the result that changes the design. The sources of the assurance model, and what this change takes from each, are in "Where the assurance model comes from" below.

### Where the assurance model comes from

The assurance parts of the design say what a claim commits the project to, what backs it, and what happens when nothing backs it. They come from six public sources. The design takes a part of each and copies none of them whole.

| Source | What the design takes | What it leaves | Where it is used |
|---|---|---|---|
| dregg, `docs/ASSURANCE.md` ([emberian/dregg](https://github.com/emberian/dregg)) | One page that a reviewer reads to know what the system guarantees, under which assumptions, and what is still open. When the page and its checked source disagree, the checked source wins | The Lean proofs and the proof system behind each guarantee | `add-codex-assurance` A4: the assurance viewpoint |
| dregg, `README-LLMs.md`, the architectural laws | Checks run on each change, and each failure is visible. A person corrects an overclaim in the text and starts its closure at the same time. A spec must be able to be false. The deployed system must meet the hypotheses of its claims | The laws that concern circuits and proofs | `add-codex-assurance` A2 and A3. The `authority` rule of P9: the model governs a guarantee |
| dregg, `GOAL-GREENS-THAT-MEAN-SOMETHING.md` | "A green only counts if it REDS when the thing it guards breaks." The ways a check passes without guarding anything. A canary for each check. The tools give a found gap to an owner at once, because nobody acts on a gap that is only in a log | The proof-engineering detail | `add-codex-assurance` A2 (canaries) and A3 (obligations) |
| Mini, `docs/TRAFFIC-PRIVACY-PROFILE-P1.md` ([emberian/minidregg](https://github.com/emberian/minidregg)) | The form of a privacy specification: what each party sees, the conditions under which a protection is absent, explicit non-goals, and what is built against what is open | The cohort mix design itself | `add-mixnet-core-specs` `design.md` |
| Cloudflare, ["How Cloudflare enforces engineering standards using AI"](https://blog.cloudflare.com/engineering-standards-enforcement/) | Normative statements with stable ids and an RFC 2119 level. A status that goes from approved (reported) to enforced (blocks a merge). Agents read statements before the full text | One statement for each SHOULD or MUST sentence: the design uses one for each requirement block | D15. `add-codex-assurance` A2: promotion needs a canary |
| Assurance cases: Goal Structuring Notation and OMG [SACM](https://www.omg.org/spec/SACM/2.3/About-SACM) | A claim, the strategy that supports it, and the evidence at each leaf. The claim must say when it does not hold | The notation, and the certification process | The `realises` chain from code to requirement to property to goal. `add-codex-assurance` A1 (warrant) |

ISO/IEC/IEEE 42010 and ArchiMate also shape the assurance viewpoint, through the terms stakeholder, concerns, viewpoint and view. RFC 2119 gives the levels of a statement.

## Decisions

### A1: Each record carries a warrant, set by the tools

| `warrant` | Meaning | Records that get it |
|---|---|---|
| `checked` | A tool reads the content from the source, and the build fails when they disagree | A code excerpt cited by symbol. A requirement whose statement has a witness, a canary, and a date on which a mutation run saw the canary red (A2) |
| `entailed` | A rule computes the content from model records | A matrix verdict computed by the rules (after A6) |
| `asserted` | A person states it, and nothing checks it | An OpenSpec requirement that lacks a witness, a canary, or a canary date. A matrix verdict typed by hand. A threat-model entity: the integrity check covers its ids, its links and the presence of its assessments, and does not cover its content |
| `narrative` | Prose that claims no truth condition | Explanation prose |

`authority` and `warrant` are two axes. A threat-model property is `normative` and can be `asserted`: the project commits to it, and nothing checks it yet. A code record is `descriptive` and is the witness for a `checked` claim.

**Measure:** the share of `normative` records for each warrant. **Changes the design:** none. This decision stores data.

### A2: A statement names its witness and its canary

A statement record gains two fields in `meta`:

| Field | Value |
|---|---|
| `witness` | The id of a test, a check, or a code record that shows the statement holds. Example: `nym/common/nymsphinx/framing/src/packet.rs#header_encoding::header_decoding_will_reject_future_versions`, the witness for the frame-version rule of `add-mixnet-core-specs` |
| `canary` | The change to the source that must turn the witness red, in one sentence, and the date a mutation run last saw it red |

A `MUST` or `SHALL` statement becomes `enforced` only when it has a witness and a canary that a mutation run has turned red. The coverage report lists statements with no witness, and statements whose witness has no canary.

The rule comes from dregg's `GOAL-GREENS-THAT-MEAN-SOMETHING.md`: "a green only counts if it REDS when the thing it guards breaks". That document lists the ways that a check can pass and guard nothing: a check that never runs, a check that compares a value with itself, a tautology, a false hypothesis, a swallowed error, and a check about code the product does not run. A canary catches the first five. For the sixth, a person must confirm that the witness covers the code path that the product uses.

**Measure:** the share of `enforced` statements whose canary turned red in its last mutation run. Target: 100%. **Changes the design:** if more than one canary in five breaks on a refactor that changes no behaviour, the witness is too close to the text of the code. Then a canary must point at a test of behaviour in place of a syntax match.

### A3: An accepted contradiction becomes an obligation with an owner

The coherence agent judges an idea against commitments and returns `supports`, `neutral`, or `contradicts`. When a person accepts a `contradicts` verdict, the tools write an obligation record. The coverage report also writes one for each `normative` record that has no witness after its owner has had it for TBD(Max) days.

An obligation record is a prose record with `meta.obligation`:

| Field | Value |
|---|---|
| `owner` | A person. The records check fails when this field is empty |
| `opened` | A date |
| `concerns` | The ids of the commitments, as `relates-to` edges |
| `closesWhen` | One sentence: the change that closes it |

The reason: dregg's rule "every caveat arrives with its closure lane already running", and its stated lesson "logging it = it never gets done". A verdict with no owner is a log entry.

**Measure:** the share of accepted `contradicts` verdicts that have an owner within 7 days, and the number of obligations closed each month. **Changes the design:** if after four weeks more than half of the obligations have no change, the obligation is a log entry with a field. Then obligations go into the issue tracker of the repository, and the record keeps only a link.

### A4: The first assurance viewpoint copies the shape of dregg's ASSURANCE.md

dregg's `docs/ASSURANCE.md` is "the single document a reviewer reads to know exactly what dregg guarantees, under what assumptions, and what is staged rather than cut over". Each guarantee in it is a theorem in a checked file, and "if they disagree, the Lean file wins".

The `assurance` viewpoint gives the Nym network the same page, made from records:

| Section | Records | Level of text |
|---|---|---|
| The guarantees | Each threat-model property, with the requirements that `realises` it and the code that realises each requirement | 0 and 1 |
| The adversaries | Each actor, with what it observes and what it cannot observe in mixnet mode | 0 |
| The assumptions | The conditions that each verdict depends on, by name | 0, when conditions are records. Until then, the section states that the corpus has no evidence |
| The open gaps | Requirements with no code, properties with no requirement, statements that are `approved` and not `enforced`, open obligations | 0 |
| Enforced against approved | Each statement with its `status`, `warrant`, witness and canary | 0 |
| Summary | A short introduction | 2, with the prompt in the viewpoint |

The draft is `viewpoints/assurance.json`, with its prompt in `viewpoints/prompts/assurance-summary.md`. The field names are provisional.

**Measure:** the stage 7 measure of `add-codex-snippets`: the share of the page that levels 0 and 1 make. **Changes the design:** if levels 0 and 1 make less than half of the page, the records lack the edges or the conditions that the page needs. Then that content work comes before any second viewpoint. Today the guarantees section is empty, because 0 of 492 requirements link to a property. `add-mixnet-core-specs` proposes the first links: 2 requirements that realise property P1, and 11 that mitigate vector V2. The view shows the second kind in its own section, "Mitigations", because a guarantee section that follows only `realises` edges hides them.

A4 proposes this viewpoint as the answer to P27 of `add-codex-snippets` (the first viewpoint, TBD(Max)), for the owner to confirm.

### A5: The sufficiency report counts the concerns that a view answers

A viewpoint lists its concerns as questions. For each concern, the viewpoint names the record kinds and facets that an answer needs. The report has two parts:

1. **Deterministic.** For each concern, the generator states whether the selection is empty. An empty selection means the view cannot answer the concern.
2. **Judged.** A person reads the view and marks each concern as answered, partly answered, or not answered. A model can propose the marks. The person decides.

The reason: fidelity to the source is necessary and not sufficient. A view can be faithful and still leave its reader's question open. The concerns are the first written statement of what a reader needs.

**Measure:** answered concerns for each view, from the judged part. **Changes the design:** if the deterministic part and the judged part disagree on more than one concern in four, the concerns are too broad to map to records. Then the owner of the viewpoint splits each concern until it names one kind of record.

### A6: The threat-model adapter takes its verdicts from the rules

An earlier experiment computes each matrix verdict from capabilities and rules, and a CI test fails when a computed verdict and a committed verdict differ. It is on a branch that is not merged or pushed yet: TBD(Max), push it or name it here. The adapter of `add-codex-snippets` reads the committed verdicts, which a person types. After the merge, the adapter reads the computed verdicts, and each verdict record is `entailed`.

**Measure:** the adapter output is identical before and after the merge, apart from the `warrant` field. **Changes the design:** if the merge changes a verdict, the rules and the committed matrix disagree. The model owner resolves each case before the merge.

### A7: An experiment tests whether the labels change what an agent does

The question: when the MCP server returns `authority` and `warrant` with each record, does an agent treat an `asserted` or `approved` statement differently from a `checked` or `enforced` one?

| Part | Content |
|---|---|
| Scenarios | About 20 tasks where the label matters. Example: "Can an integrator rely on cover traffic to hide when a client is idle?" The correct answer depends on whether the statement is `enforced` and whether the integrator can turn the stream off |
| Arms | The same MCP server, with the labels in each response and with the labels removed |
| Reading | A person reads each transcript and marks whether the agent stated the limit that the label implies |
| Sets | The stage 0 labelled query sets of `add-codex-snippets`, plus the scenarios above |

**Measure:** the share of scenarios where the agent states the limit, in each arm. **Changes the design:** if the arms do not differ, the labels stay in the records for people and for the coherence agent, and the MCP server stops sending them.

## Risks

| Risk | Answer |
|---|---|
| Canaries become a second copy that goes out of date | A canary is one sentence and a date. The mutation run checks it. A canary that the run cannot apply fails the run |
| The number of open obligations grows | A3 states the result that moves them to the issue tracker |
| The assurance page looks complete while the graph is empty | Each section with no input states that the corpus has no evidence. The open gaps section lists the empty parts first |
| A judged sufficiency mark is one person's opinion | The deterministic part runs first. The judged part records who marked it |

## Follow-ups (not this change)

- A second assurance viewpoint for a reader who asks "should I trust it", after dregg's `docs/TRUST-LEVELS.md`.
- Conditions as records, so that the assumptions section can have level 0 text.
- A relation for obligations, when one real edge needs a meaning that `relates-to` does not have.
