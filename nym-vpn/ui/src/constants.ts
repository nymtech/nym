export const routes = {
  root: '/',
  settings: '/settings',
  entryNodeLocation: '/entry-node-location',
  exitNodeLocation: '/exit-node-location',
} as const;

export const AppName = 'NymVPN';
export const ConnectionEvent = 'connection-state';
export const ProgressEvent = 'connection-progress';
export const QuickConnectPrefix = 'Fastest';
export const DefaultNodeLocation = { name: 'France', code: 'FR' };
