/** Max inner content width for wide pages (Balance, etc.). Must match historical PageLayout default. */
export const CONTENT_RAIL_MAX_WIDTH_WIDE = 1280;

/** Narrow variant (when PageLayout maxWidth="narrow"). */
export const CONTENT_RAIL_MAX_WIDTH_NARROW = 1000;

export type ContentRailWidth = number | 'narrow' | 'wide';

export function resolveContentRailMaxWidth(maxWidth?: ContentRailWidth): number {
  if (maxWidth === 'narrow') {
    return CONTENT_RAIL_MAX_WIDTH_NARROW;
  }
  if (maxWidth === 'wide' || maxWidth === undefined) {
    return CONTENT_RAIL_MAX_WIDTH_WIDE;
  }
  return maxWidth;
}
