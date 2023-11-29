import { PrivacyMode } from './app-state';

export type UiMode = 'Dark' | 'Light';

export interface NodeConfig {
  id: string;
  country: string;
}

export interface AppData {
  monitoring: boolean;
  autoconnect: boolean;
  killswitch: boolean;
  uiMode: UiMode;
  privacyMode: PrivacyMode;
  entryNode?: NodeConfig | null;
  exitNode?: NodeConfig | null;
}

// tauri type, hence the use of snake_case
export interface AppDataFromBackend {
  monitoring: boolean | null;
  autoconnect: boolean | null;
  killswitch: boolean | null;
  ui_mode: UiMode | null;
  privacy_mode: PrivacyMode;
  entry_node: NodeConfig | null;
  exit_node: NodeConfig | null;
}
