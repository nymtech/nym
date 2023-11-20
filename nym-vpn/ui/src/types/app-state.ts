type ConnectionState =
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
  privacyMode: PrivacyMode;
  tunnel: TunnelConfig;
};
