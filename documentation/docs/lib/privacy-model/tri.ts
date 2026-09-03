// Pure tri-state → glyph/class helpers, usable from both server and client
// components (kept out of any "use client" module).

import type { Tri } from "./types";

export function triGlyph(t: Tri): string {
  return t === "yes" ? "✅" : t === "no" ? "❌" : "◐";
}
export function triClass(t: Tri): string {
  return t === "yes" ? "tri-yes" : t === "no" ? "tri-no" : "tri-partial";
}
