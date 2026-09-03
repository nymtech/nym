# Simplified Technical English (descriptive register)

A contributor reference for the network and developer docs. Not a published page.
Based on ASD-STE100, adapted to a **descriptive** register: these are Explanation
and Reference pages, not procedures, so the "one instruction per sentence" rule
does not apply. The goal is clear, unambiguous prose that reads the same to every
reader.

## Sentence rules

- **Length.** Keep description sentences to 25 words or fewer. Keep any
  procedural step to 20 words or fewer. Split a long sentence into two.
- **One idea per sentence.** Avoid nested subordinate clauses. If a sentence has
  two "and" clauses that each carry a distinct idea, make two sentences.
- **Active voice.** "The exit gateway sees the destination", not "the destination
  is seen by the exit gateway".
- **Present tense.** Describe how the system behaves now.
- **No "-ing" pile-ups.** Rewrite "hiding your IP by relaying, then padding" as
  short clauses or separate sentences.

## Vocabulary (one word, one meaning)

Use one term for one concept, every time.

| Use | Not |
|---|---|
| destination | server, endpoint, host (when meaning L2) |
| exit gateway | exit node, exit relay |
| client | user, wallet, app (when meaning the Nym client) |
| mixnet | mix network, the mix |
| request | call, query, message (when meaning a protocol interaction) |
| hide | obscure, mask, cloak |
| observe / see | witness, glimpse, catch |
| unlinkable | untraceable, anonymous (P1/P2 are precise; do not swap) |

Approved verbs for describing behaviour: **see, observe, hide, protect, route,
forward, strip, add, remove, link, attribute, close (a vector), reveal, leak**.
Prefer these over near-synonyms.

## Words to avoid

- **Vague quantifiers:** "very", "quite", "somewhat", "a lot". State the
  condition instead ("weakens with low network traffic").
- **Marketing verbs:** "leverage", "empower", "seamless", "robust", "cutting-edge".
- **Ambiguous "it/this"** with no clear antecedent. Name the thing.
- **shall.** Use "must" for a requirement and "must not" for a prohibition.

## House style (already enforced elsewhere)

- British spellings: behaviour, colour, minimise, unauthorised.
- No em-dashes and no `--` in prose or comments. Use commas, colons, parentheses,
  or full stops.
- No decorative `---` or `===` dividers in code comments.
- No emoji in docs (functional glyphs such as the matrix verdicts are fine).
- Articles: always use "the" or "a"; do not drop them for brevity.

## Terminology anchors (threat model)

- Actors: **L1** (public observer), **L2** (the destination), **L3L** (local
  network observer), **L3G** (global network observer).
- Vectors: **V1** session state, **V2** timing, **V3** content.
- Properties: **P1** request-identity unlinkability, **P2** request-request
  unlinkability.
- Layers: **transport** (hide who you talk to, from whom) and **hygiene**
  (discipline what your request pattern leaks to the destination).
Use these labels exactly; link the first use on a page to the actors/vectors/
properties reference.
