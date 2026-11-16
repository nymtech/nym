// The shapes of the archetype files (runbook.json, tutorial.json), of
// placeholders.json, and of a lint rule.

import type { Node, Root } from './mdast.mts';

export interface ArchetypeSection {
  name: string;
  required: boolean;
  /** A heading at this depth, with the text `heading` when one is given. */
  depth?: number;
  heading?: string;
  /** A JSX element, such as `Steps`, in place of a heading. */
  element?: string;
}

export interface Archetype {
  sections: ArchetypeSection[];
  languages: string[];
  /** A wrong language and the name to use in its place, such as `shell` -> `sh`. */
  languageFixes: Record<string, string>;
  metaKeys: string[];
  /** The level of each rule, by id: `warn` or `error`. */
  rules: Record<string, string>;
}

export interface Placeholder {
  /** The token, such as `<PUBLIC_IP>`. Two entries (from the old variables.csv) have a flag and no name. */
  name?: string;
  flag?: string;
  description: string;
  example?: string;
  aliases?: string[];
}

export interface PlaceholderIndex {
  /** Each name and alias, to its entry. */
  byToken: Map<string, Placeholder>;
}

export type Report = (node: Node, message: string, fix?: string) => void;

export interface RuleContext {
  report: Report;
  archetype: Archetype;
  placeholders: PlaceholderIndex;
}

export interface Rule {
  id: string;
  check: (tree: Root, context: RuleContext) => void;
}

export type Level = 'warn' | 'error';

export interface Fault {
  rule: string;
  level: Level;
  file: string;
  line: number;
  message: string;
  fix?: string;
}

/** A problem with the page itself, not a rule fault: an unknown type, or an import that does not resolve. */
export interface LintWarning {
  file: string;
  line: number | undefined;
  message: string;
}
