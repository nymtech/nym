import { describe, it, expect } from 'vitest';
import type { Connection } from '@nymproject/retrieval-indexer/store';
import { retryDroppedConnection, storeSource } from './retrieval-source';

const row = { id: 'install', source: 'nym-docs', title: 'Quick Start', heading: 'Install', url: 'https://nym.com/docs/install', text: 'run npm i', lang: null, tokens_est: 3, test: false, score: 0.9 };

// Answers the package's read-side statements from shard models the test can
// change between calls, and records the LIMIT each search asked for.
function fakeStore(models: Record<string, { model: string; dim: number }>) {
  const limits: unknown[] = [];
  const db: Connection = {
    async query(text: string, params?: unknown[]) {
      if (text.includes('from shards')) {
        const m = models[params![0] as string];
        return { rows: (m ? [m] : []) as any[] };
      }
      limits.push(params![3]);
      return { rows: [row] as any[] };
    },
  };
  return { db, limits, models };
}

const queryModels = { docs: 'voyage-3-large', code: 'voyage-code-3' };
const vec = [1, 0];

describe('storeSource', () => {
  it('searches a modality whose rows match the query model, with the given topK', async () => {
    const { db, limits } = fakeStore({ docs: { model: 'voyage-3-large', dim: 2 } });
    const hits = await storeSource(db, queryModels).searchDocs(vec, 4);
    expect(hits.map((h) => h.chunk.id)).toEqual(['install']);
    expect(limits).toEqual([4]);
  });

  it('refuses a modality embedded with another model', async () => {
    const { db } = fakeStore({ docs: { model: 'mock', dim: 2 } });
    await expect(storeSource(db, queryModels).searchDocs(vec, 6)).rejects.toThrow(/embedded with mock, but the query is embedded with voyage-3-large/);
  });

  it('reports a missing shard as an error, not as no results', async () => {
    const { db } = fakeStore({ docs: { model: 'voyage-3-large', dim: 2 } });
    await expect(storeSource(db, queryModels).searchCode!(vec, 6)).rejects.toThrow(/no code shard/);
  });

  it('checks the model on every search, so a later re-embed is caught', async () => {
    const { db, models } = fakeStore({ docs: { model: 'voyage-3-large', dim: 2 } });
    const source = storeSource(db, queryModels);
    await source.searchDocs(vec, 6);
    models.docs = { model: 'voyage-3-lite', dim: 2 };
    await expect(source.searchDocs(vec, 6)).rejects.toThrow(/embedded with voyage-3-lite/);
  });
});

describe('retryDroppedConnection', () => {
  // Fails the first call with `error`, then answers.
  function flaky(error: string) {
    let calls = 0;
    const db: Connection = {
      async query() {
        calls++;
        if (calls === 1) throw new Error(error);
        return { rows: [{ ok: true }] as any[] };
      },
    };
    return { db, calls: () => calls };
  }

  it('retries once when the connection was dropped', async () => {
    const { db, calls } = flaky('Connection terminated unexpectedly');
    expect((await retryDroppedConnection(db).query('select 1')).rows).toEqual([{ ok: true }]);
    expect(calls()).toBe(2);
  });

  it('does not retry a timeout or a query error', async () => {
    for (const error of ['Query read timeout', 'relation "chunks" does not exist']) {
      const { db, calls } = flaky(error);
      await expect(retryDroppedConnection(db).query('select 1')).rejects.toThrow(error);
      expect(calls()).toBe(1);
    }
  });
});
