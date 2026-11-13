## Why

`add-codex-snippets` gives the corpus records, typed edges, an `authority` on each record, and statements with a `status`. It stops at the records. This change uses them for four jobs that the records make possible but that no task builds:

1. **One page that says what the network guarantees.** A reviewer has no single page that lists each guarantee, the assumptions it rests on, and the gaps that are still open. The records hold all three. A generator can make the page from a viewpoint.
2. **Witnesses that can fail.** A statement has a `level` and a `status`, but no record says what shows that it holds, or whether that check can fail. A check that cannot fail is a constant. It looks like evidence but proves nothing.
3. **Verdicts that lead to work.** The coherence agent returns a verdict that cites commitments. A verdict with no owner becomes a log entry, and nobody acts on a log entry.
4. **A measure of sufficiency.** A record can be faithful to its source and still not answer the question of its reader. A viewpoint lists the reader's questions, so a tool can count the questions that a view answers.

A fifth item joins two pieces of work. An earlier experiment computes the threat-model verdicts from rules and gates them in CI. It is on a branch that is not merged or pushed yet. `add-codex-snippets` turns the same model into records, and its adapter reads verdicts that a person types. One piece therefore joins the rules to the surfaces, and the other joins the model to the records. Neither has both.

One outside project shows the same practice. dregg (`emberian/dregg`) keeps one assurance document whose authority is a machine-checked file, and its rule for checks is "a green only counts if it REDS when the thing it guards breaks". Its rule for gaps is that each caveat comes with the work that closes it already started.

## What Changes

- Add a `warrant` to each record in `meta`: `checked`, `entailed`, `asserted`, or `narrative`. The tools set it from the way that they make the record. `authority` says what a record commits the project to. `warrant` says what backs it. The two are independent.
- Add a `witness` and a `canary` to a statement in `meta`. The witness names the test or check that shows the statement holds. The canary names the change to the source that must turn the witness red. A statement with the level `MUST` or `SHALL` can become `enforced` only with a canary that a mutation run has turned red.
- Extend the coverage report with two lists: statements with no witness, and statements whose witness has no canary.
- Add obligation records. When a person accepts a `contradicts` verdict, or when the coverage report shows a gap on a `normative` record, the tools write a prose record with `meta.obligation`: an owner, the date it opened, the ids it concerns, and the condition that closes it. An obligation with no owner fails the records check.
- Add the first assurance viewpoint, `assurance`, in the shape of dregg's `docs/ASSURANCE.md`: the guarantees, the assumptions by name, the open gaps, and what is enforced against what is only approved. A draft viewpoint file is in `viewpoints/`.
- Add a sufficiency report for each viewpoint: for each concern, whether the view has records that answer it.
- Bring the rules-based verdicts of the rules experiment, on a branch that is not merged or pushed yet (TBD(Max): push it or name it here) into the threat-model adapter, so that a matrix verdict is `entailed` and not `asserted`.
- Define an experiment that tests whether the `authority` and `warrant` labels change what an agent does.

## Capabilities

### New Capabilities

- `codex-assurance`: the warrant field, statement witnesses and canaries, obligation records, the assurance viewpoint, and the sufficiency report.

### Modified Capabilities

None. The record format keeps its fields. The new fields are in `meta`.

## Impact

- `documentation/docs/lib/snippets/`: the record builder sets `warrant`. The coverage report gains two columns. The records check gains the obligation rule.
- `.nym-codex/manifest.json`: the list of enforced statements gains the canary rule.
- The codex site: the `assurance` viewpoint file and its template. The generator is stage 7 of `add-codex-snippets` and does not exist yet.
- The threat-model adapter: verdicts computed from rules, after the merge of the two branches.
- No change to product code.

## Placeholders

Owners and thresholds are not known yet. A `TBD(<who>)` marker in `design.md` and `tasks.md` shows each one.

## Non-Goals

- The coherence agent itself. `add-codex-snippets` designs it, and this change defines only what happens to its verdicts.
- A review agent that blocks merges. `enforced` keeps the meaning that `add-codex-snippets` gives it.
- New edge relations. Obligation records use `relates-to` until a new relation has a stated meaning and one real edge.
- The mixnet core specs. `add-mixnet-core-specs` writes them, and this change depends on them for the assurance view.
