import { Dispatch } from 'react';
import { AppData } from './app-data';
import { StateAction } from '../state';

export type ConnectionState =
  | 'Connected'
  | 'Disconnected'
  | 'Connecting'
  | 'Disconnecting'
  | 'Error';

export type PrivacyMode = 'High' | 'Medium' | 'Low';

export interface TunnelConfig {
  id: string;
  name: string;
}

export type AppState = {
  state: ConnectionState;
  loading: boolean;
  privacyMode: PrivacyMode;
  tunnel: TunnelConfig;
  uiMode: 'Light' | 'Dark';
  localAppData: AppData;
};

export type EventPayload = {
  state: ConnectionState;
};

export type StateDispatch = Dispatch<StateAction>;
