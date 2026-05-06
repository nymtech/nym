import createCache from '@emotion/cache';

/**
 * WKWebView (Tauri) can silently fail Emotion's production `insertRule` path (`speedy: true`),
 * which shows up as React/MUI rendering with almost no CSS while scripts run.
 * `speedy: false` uses `<style>` node insertion, which is reliable here.
 */
export const muiEmotionCache = createCache({
  key: 'mui',
  prepend: true,
  speedy: false,
});
