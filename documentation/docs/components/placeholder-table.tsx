// The variables table of `content/operators/variables.mdx` (task 9.14), made from
// `lib/archetypes/placeholders.json`, the list that the runbook lint checks the
// `<...>` tokens against, so the page and the lint read one list.

import { Anchor, Code, Table } from "nextra/components";
import type { ReactNode } from "react";
import placeholders from "../lib/archetypes/placeholders.json";

interface Placeholder {
  name?: string;
  flag?: string;
  description: string;
  example?: string;
}

// A description holds inline Markdown: `code` spans and [text](url) links.
const INLINE = /(`[^`]+`|\[[^\]]+\]\([^)]+\))/;
const LINK = /^\[([^\]]+)\]\(([^)]+)\)$/;

function inline(text: string): ReactNode[] {
  return text.split(INLINE).map((part, i) => {
    if (part.startsWith("`")) return <Code key={i}>{part.slice(1, -1)}</Code>;
    const link = part.match(LINK);
    if (link) return <Anchor key={i} href={link[2]}>{link[1]}</Anchor>;
    return part;
  });
}

// The flags with no variable first, as in the table this replaces, then by name.
const ROWS = [...(placeholders as Placeholder[])].sort(
  (a, b) => Number(Boolean(a.name)) - Number(Boolean(b.name)) || (a.name ?? a.flag ?? "").localeCompare(b.name ?? b.flag ?? ""),
);

export function PlaceholderTable() {
  return (
    <Table>
      <thead>
        <Table.Tr>
          <Table.Th>Flag (Option)</Table.Th>
          <Table.Th>Variable</Table.Th>
          <Table.Th>Description</Table.Th>
          <Table.Th>Syntax example</Table.Th>
        </Table.Tr>
      </thead>
      <tbody>
        {ROWS.map((row) => (
          <Table.Tr key={row.name ?? row.flag}>
            <Table.Td>{row.flag ? <Code>{row.flag}</Code> : null}</Table.Td>
            <Table.Td>{row.name ? <Code>{row.name}</Code> : <em>none</em>}</Table.Td>
            <Table.Td>{inline(row.description)}</Table.Td>
            <Table.Td>{row.example ? <Code>{row.example}</Code> : null}</Table.Td>
          </Table.Tr>
        ))}
      </tbody>
    </Table>
  );
}
