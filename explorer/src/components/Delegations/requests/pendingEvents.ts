import { PendingEpochEvent } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const getPendingEpochEvents = async () => invokeWrapper<PendingEpochEvent[]>('get_pending_epoch_events');
