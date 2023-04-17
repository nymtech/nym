import { invokeWrapper } from './wrapper';
import { AppVersion } from '../types/rust/AppVersion';

export const checkVersion = async () => invokeWrapper<AppVersion>('check_version');
