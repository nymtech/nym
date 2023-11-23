import { PrivacyMode } from './app-state.ts';

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

export type AppDataFromStorage = Partial<AppData>;
