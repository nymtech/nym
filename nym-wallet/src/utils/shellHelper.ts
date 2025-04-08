import { open } from '@tauri-apps/plugin-shell';

export const openInBrowser = async (url: string): Promise<void> => {
  try {
    await open(url);
  } catch (error) {
    // Error handling silenced to comply with no-console rule
    try {
      window.open(url, '_blank');
    } catch (e) {
      // Error handling silenced to comply with no-console rule
    }
  }
};
