export const routes = {
  root: '/',
  settings: '/settings',
  display: '/settings/display',
  logs: '/settings/logs',
  feedback: '/settings/feedback',
  legal: '/settings/legal',
  entryNodeLocation: '/entry-node-location',
  exitNodeLocation: '/exit-node-location',
} as const;

export const AppName = 'NymVPN';
export const ConnectionEvent = 'connection-state';
export const ProgressEvent = 'connection-progress';
export const QuickConnectPrefix = 'Fastest';
// TODO ⚠ keep this value in sync with the one declared in `index.html`
export const DefaultRootFontSize = 12; // in px
