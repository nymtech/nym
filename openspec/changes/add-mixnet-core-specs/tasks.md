# Tasks

This change is task 6e.5 of `add-codex-snippets`. Owner: TBD(stage 5 owner).

## 1. Review the specs

- [ ] 1.1 A maintainer of `common/nymsphinx` reviews `sphinx-packet-format`
- [ ] 1.2 A maintainer of `common/client-core` reviews `client-cover-traffic`
- [ ] 1.3 A maintainer of `nym-node` reviews `mix-node-delay`
- [ ] 1.4 Run `openspec validate add-mixnet-core-specs --strict`

## 2. Threat-model links

- [ ] 2.1 The threat-model owner reviews each edge in `edges.proposed.json`, and each requirement listed as serving no entity
- [ ] 2.2 Move the accepted edges into `documentation/docs/.nym-codex/edges.json` after the change is archived, so that the requirement ids exist. The `none` list has no place in that file's format (D4c). Task 6e.3 of `add-codex-snippets` decides where a stated none lives
- [ ] 2.3 Run the coverage report. Record the count of requirements with a threat-model link, and of complete chains

## 3. Findings

- [ ] 3.1 Give each finding in `design.md` to the owner of its code. Record the answer next to the finding

## 4. Witnesses

- [ ] 4.1 For each requirement with no test, decide whether it needs one before its statement can become `enforced` (`add-codex-assurance`, A2). Start with the replay drop, the three-mix route, and the cover-slot behaviour of the main stream
