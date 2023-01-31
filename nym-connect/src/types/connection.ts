export enum ConnectionStatusKind {
  disconnected = 'disconnected',
  disconnecting = 'disconnecting',
  connected = 'connected',
  connecting = 'connecting',
}

export type GatewayPerformance = 'Good' | 'Poor' | 'VeryPoor';
