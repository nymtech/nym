import { VpnMode } from './app-state';

export type UiTheme = 'Dark' | 'Light';

export interface NodeConfig {
  id: string;
  country: string;
}

export interface AppData {
  monitoring: boolean;
  autoconnect: boolean;
  killswitch: boolean;
  uiTheme: UiTheme;
  vpnMode: VpnMode;
  entryNode?: NodeConfig | null;
  exitNode?: NodeConfig | null;
}

// tauri type, hence the use of snake_case
export interface AppDataFromBackend {
  monitoring: boolean | null;
  autoconnect: boolean | null;
  killswitch: boolean | null;
  ui_theme: UiTheme | null;
  vpn_mode: VpnMode;
  entry_node: NodeConfig | null;
  exit_node: NodeConfig | null;
  node_countries: string[];
}
