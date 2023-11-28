import { Dispatch } from 'react';
import { AppData } from './app-data';
import { StateAction } from '../state';

export type ConnectionState =
  | 'Connected'
  | 'Disconnected'
  | 'Connecting'
  | 'Disconnecting'
  | 'Unknown';

export type PrivacyMode = 'High' | 'Medium' | 'Low';

export interface TunnelConfig {
  id: string;
  name: string;
}

export type AppState = {
  state: ConnectionState;
  loading: boolean;
  error?: string | null;
  progressMessages: string[];
  privacyMode: PrivacyMode;
  tunnel: TunnelConfig;
  uiMode: 'Light' | 'Dark';
  localAppData: AppData;
};

export type ConnectionEventPayload = {
  state: ConnectionState;
  error?: string | null;
};

export type ProgressEventPayload = {
  message: string;
};

export type StateDispatch = Dispatch<StateAction>;
