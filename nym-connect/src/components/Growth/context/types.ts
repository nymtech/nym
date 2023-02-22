import { DateTime } from 'luxon';

export interface ClientId {
  client_id: string;
  client_id_signature: string;
}

export interface Registration {
  id: string;
  client_id: string;
  client_id_signature: string;
  timestamp: string;
}

export interface DrawEntryPartial {
  draw_id: string;
  client_id: string;
  client_id_signature: string;
}

export enum DrawEntryStatus {
  pending = 'Pending',
  winner = 'Winner',
  noWin = 'NoWin',
  claimed = 'Claimed',
}

export interface DrawEntry {
  id: string;
  draw_id: string;
  timestamp: string;
  status: DrawEntryStatus;
}

export interface DrawWithWordOfTheDay {
  id: string;
  start_utc: string;
  end_utc: string;
  word_of_the_day?: string;
  last_modified: string;
  entry?: DrawEntry;
}

export interface ClaimPartial {
  draw_id: string;
  registration_id: string;
  client_id: string;
  client_id_signature: string;
  wallet_address: string;
}

export interface Winner {
  id: string;
  client_id: string;
  draw_id: string;
  timestamp: string;
  winner_reg_id: string;
  winner_wallet_address?: string;
  winner_claim_timestamp?: string;
}

export interface Draws {
  current?: DrawWithWordOfTheDay;
  next?: DrawWithWordOfTheDay;
  draws: DrawEntry[];
}
