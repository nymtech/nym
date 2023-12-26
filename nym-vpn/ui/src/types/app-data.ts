import { VpnMode } from './app-state';

export type UiTheme = 'Dark' | 'Light';

export interface NodeConfig {
  id: string;
  country: Country;
}

export type Country = {
  name: string;
  code: string;
};

// tauri type, hence the use of snake_case
export interface AppDataFromBackend {
  monitoring: boolean | null;
  autoconnect: boolean | null;
  killswitch: boolean | null;
  entry_location_selector: boolean | null;
  ui_theme: UiTheme | null;
  ui_root_font_size: number | null;
  vpn_mode: VpnMode | null;
  entry_node: NodeConfig | null;
  exit_node: NodeConfig | null;
  entry_node_location: Country | null;
  exit_node_location: Country | null;
}
