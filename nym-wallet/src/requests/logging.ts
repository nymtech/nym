import { invokeWrapper } from './wrapper';

export const helpLogToggleWindow = async () => invokeWrapper<void>('help_log_toggle_window', {});

export const logViewerWindowSupported = async () =>
  invokeWrapper<boolean>('log_viewer_window_supported', {});
