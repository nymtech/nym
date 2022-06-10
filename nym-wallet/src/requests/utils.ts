import { AppEnv } from 'src/types';
import { invokeWrapper } from './wrapper';

export const getEnv = async () => invokeWrapper<AppEnv>('get_env');
