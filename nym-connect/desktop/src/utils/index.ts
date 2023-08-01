import { invoke } from '@tauri-apps/api';

export const toggleLogViewer = async () => {
  await invoke('help_log_toggle_window');
};
