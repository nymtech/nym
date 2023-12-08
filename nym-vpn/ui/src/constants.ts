import { Country } from './types';

export const routes = {
  root: '/',
  settings: '/settings',
  entryNodeLocation: '/entry-node-location',
  exitNodeLocation: '/exit-node-location',
} as const;

export const AppName = 'NymVPN';
export const ConnectionEvent = 'connection-state';
export const ProgressEvent = 'connection-progress';
//putting this here for now until decided how default country is determined
export const QuickConnectPrefix = 'Recommended';
export const QuickConnectCountry: Country = {
  name: `${QuickConnectPrefix} (Germany)`,
  code: 'DE',
};
