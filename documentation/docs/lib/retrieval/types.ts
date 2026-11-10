// Snippet records: the contract between a source repository and each consumer
// (codex, the docs site, the index). See openspec/changes/add-codex-snippets.
//
// The retrieval index types (pages, chunks, the index file) live in
// @nymproject/retrieval-indexer (types).

export type SnippetKind = "prose" | "code" | "diagram" | "model";

/** A directed edge from this snippet to another one. A closed list. */
export type RefRel =
  | "cites" // the body embeds the target with <Snippet id>
  | "relates-to" // the default for a reference with no relation, and for a link
  | "depends-on" // the reader needs the target to understand this snippet
  | "realises" // this snippet carries out the target: code a requirement, a requirement a property
  | "specialises" // this snippet is a particular case of the target
  | "observes" // model: an actor, to a vector it can observe
  | "mitigates" // this snippet reduces the target vector
  | "assessed-against"; // model: a scenario, to an actor or a property

/**
 * Which records are commitments (P9, decided 2026-10-16): the OpenSpec
 * requirements and the threat model are normative, a specification or an
 * explanation describes, and a guide (a how-to or a tutorial) illustrates.
 */
export type Authority = "normative" | "descriptive" | "illustrative";

export interface SnippetRef {
  id: string;
  rel: RefRel;
}

export interface Snippet {
  id: string; // unique across all repositories
  kind: SnippetKind;
  authority: Authority;
  repo: string; // "nym"
  path: string; // the snippet file, or for code the source file
  lines?: [number, number]; // code: 1-indexed, inclusive, in `path`
  title: string; // prose: the heading text, or the frontmatter title. diagram: the file name. code: the symbol
  lang?: string; // code: "rust". diagram: "mermaid"
  body: string; // prose: the text below the heading. diagram, code: the source text. model: the text projection
  hash: string; // sha256 of `body`
  refs: SnippetRef[]; // typed edges, sorted by id then rel
  meta: Record<string, unknown>; // other frontmatter, passed through
}
