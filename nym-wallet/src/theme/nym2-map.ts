// Wire 3: Nym 2.0 token → wallet MUI palette mapping — APPLIED 2026-06-03.
// All six changes confirmed by Hux and applied to theme.tsx.
// Reference: nym2-tokens.ts (sourced from nym-color-tokens_fin.html, 2026-05-29).

import { nym2Dark, nym2Light } from './nym2-tokens';

// Applied to theme.tsx (dark mode):
export const walletPaletteUpdates = {
  highlight: nym2Dark.primary, // #5BF0A0 — primary green
  backgroundMain: nym2Dark.background, // #0A0A0A — near-black
  backgroundPaper: nym2Dark.surface, // #1A1A1C
  info: nym2Dark.info, // #485ECA — teal → indigo
  error: nym2Dark.error, // #E73E14
  textSubdued: nym2Dark.textSecondary, // #AEACB1
} as const;

// Light mode equivalents — not yet applied (wallet is dark-mode-only for MVP).
// Apply when light theme ships.
export const walletLightUpdates = {
  // highlight:       nym2Light.primary,       // #1ED674
  // backgroundMain:  nym2Light.background,    // #FFFFFF
  // backgroundPaper: nym2Light.surface,       // #F5F5FA
  // info:            nym2Light.info,          // #485ECA
  // error:           nym2Light.error,         // #E73E14
  // textSubdued:     nym2Light.textSecondary, // #5A5A60
} as const;

// Not applied — separate semantic from primary green.
// nymPalette.success left at rgb(20, 231, 111) pending design decision on
// primary vs success distinction (Nym 2.0 success = #28C96C, different hue).
