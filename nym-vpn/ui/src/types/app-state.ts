import { Dispatch } from 'react';
import { Dayjs } from 'dayjs';
import { StateAction } from '../state';
import { AppData } from './app-data';

export type ConnectionState =
  | 'Connected'
  | 'Disconnected'
  | 'Connecting'
  | 'Disconnecting'
  | 'Unknown';

export type VpnMode = 'TwoHop' | 'Mixnet';

export interface TunnelConfig {
  id: string;
  name: string;
}

export type AppState = {
  state: ConnectionState;
  loading: boolean;
  error?: string | null;
  progressMessages: string[];
  sessionStartDate?: Dayjs | null;
  vpnMode: VpnMode;
  tunnel: TunnelConfig;
  uiMode: 'Light' | 'Dark';
  localAppData: AppData;
};

export type ConnectionEventPayload = {
  state: ConnectionState;
  error?: string | null;
  start_time?: number | null; // unix timestamp in seconds
};

export type ProgressEventPayload = {
  message: string;
};

export type StateDispatch = Dispatch<StateAction>;
