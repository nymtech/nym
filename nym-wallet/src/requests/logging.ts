import { invokeWrapper } from './wrapper';

export const helpLogToggleWindow = async () => invokeWrapper<void>('help_log_toggle_window', {});
