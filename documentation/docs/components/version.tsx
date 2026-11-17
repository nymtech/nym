// `<Version name="RUST_MSRV" />`: a constant from components/versions.ts in the
// text. A snippet imports nothing, so it names a version this way rather than
// with `{RUST_MSRV}`. `code` shows it as inline code, which `{X}` inside
// backticks cannot do: MDX does not evaluate an expression in inline code.
// The text outputs substitute the same value (lib/retrieval/mdx.mjs, `substitute`).

import { Code } from "nextra/components";
import * as VERSIONS from "./versions";

export function Version({ name, code = false }: { name: string; code?: boolean }) {
  const value = (VERSIONS as Record<string, unknown>)[name];
  if (typeof value !== "string" && typeof value !== "number") {
    throw new Error(`<Version name="${name}">: components/versions.ts has no such constant`);
  }
  return code ? <Code>{String(value)}</Code> : <>{String(value)}</>;
}
