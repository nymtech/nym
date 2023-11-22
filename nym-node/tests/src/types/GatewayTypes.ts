export interface ClientInterfaces {
  mixnet_websockets: MixnetWebsockets;
  wireguard: Wireguard;
}

export interface MixnetWebsockets {
  ws_port: number | null;
  wss_port: number | null;
}

export interface Wireguard {
  port: number | null;
  public_key: string;
}
