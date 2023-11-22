export interface BuildInformation {
  binary_name: string;
  build_timestamp: string;
  build_version: string;
  cargo_profile: string;
  commit_branch: string;
  commit_sha: string;
  commit_timestamp: string;
  rustc_channel: string;
  rustc_version: string;
}

export interface HostInformation {
  data: Data;
  signature: string;
}

export interface Data {
  hostname: string | null;
  ip_address: string[];
  keys: Keys;
}

export interface Keys {
  ed25519: string;
  x25519: string;
}

export interface Roles {
  gateway_enabled: boolean;
  mixnode_enabled: boolean;
  network_requester_enabled: boolean;
}
