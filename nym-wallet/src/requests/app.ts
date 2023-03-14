import { invokeWrapper } from './wrapper';
import { AppVersion } from '../types/rust/AppVersion';

export const checkVersion = async () => invokeWrapper<AppVersion>('check_version');

export const createMainWindow = async (): Promise<void> => invokeWrapper<void>('create_main_window');

export const createSignInWindow = async (): Promise<void> => invokeWrapper<void>('create_auth_window');

export const setReactState = async (newState?: string): Promise<void> =>
  invokeWrapper<void>('set_react_state', { newState });

export const getReactState = async (): Promise<string | undefined> =>
  invokeWrapper<string | undefined>('get_react_state');
