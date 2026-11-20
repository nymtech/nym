// Where the MCP tools read the index from: the files the docs build bundles into
// the function, or the public Postgres store (task 5.2 of the
// add-shared-retrieval-index change). The tools see only this interface.

import type { DocIndex } from '@nymproject/retrieval-indexer/types';
import type { Connection, Modality } from '@nymproject/retrieval-indexer/store';
import { search as storeSearch, getSection as storeGetSection } from '@nymproject/retrieval-indexer/search';
import { search as indexSearch, getSection as indexGetSection } from './retrieval';

export interface RetrievedChunk {
  id: string;
  title: string;
  heading: string;
  url: string;
  text: string;
}

export interface RetrievedHit {
  chunk: RetrievedChunk;
  score: number;
}

export interface RetrievalSource {
  searchDocs(queryVector: number[], topK: number): Promise<RetrievedHit[]>;
  getSection(ref: string): Promise<RetrievedChunk | null>;
  /** Absent when this source has no code index; search_code is then not exposed. */
  searchCode?: (queryVector: number[], topK: number) => Promise<RetrievedHit[]>;
}

export function indexSource(index: DocIndex, codeIndex?: DocIndex): RetrievalSource {
  return {
    searchDocs: async (queryVector, topK) => indexSearch(queryVector, index, { topK }),
    getSection: async (ref) => indexGetSection(index, ref),
    ...(codeIndex ? { searchCode: async (queryVector: number[], topK: number) => indexSearch(queryVector, codeIndex, { topK }) } : {}),
  };
}

/**
 * Each search names the model the route embedded its query with, so the package
 * refuses a modality with no shard, or rows of another model, on every call: an
 * error the agent can read, not an empty or wrong result.
 *
 * The store source always offers code search. With no code shard, search_code
 * returns that error.
 */
export function storeSource(db: Connection, queryModels: Record<Modality, string>): RetrievalSource {
  const searchModality = (modality: Modality) => (queryVector: number[], topK: number) =>
    storeSearch(db, modality, queryVector, { topK, model: queryModels[modality] });
  return {
    searchDocs: searchModality('docs'),
    getSection: (ref) => storeGetSection(db, ref),
    searchCode: searchModality('code'),
  };
}

// A frozen serverless instance runs no timers, so the pool cannot drop a socket
// that the pooler or a NAT closed meanwhile. The first query after the thaw then
// fails on it. Retrying once gets a fresh connection. Timeouts are not retried:
// a store that does not answer would only make the agent wait twice as long.
const DROPPED_CONNECTION = /ECONNRESET|EPIPE|Connection terminated/i;

export function retryDroppedConnection(db: Connection): Connection {
  return {
    async query<T = Record<string, unknown>>(text: string, params?: unknown[]) {
      try {
        return await db.query<T>(text, params);
      } catch (e) {
        if (!DROPPED_CONNECTION.test((e as Error).message)) throw e;
        return db.query<T>(text, params);
      }
    },
  };
}
