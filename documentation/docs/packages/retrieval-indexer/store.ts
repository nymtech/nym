// Postgres + pgvector store for retrieval shards (design decisions 1-3).
//
// A shard is the chunks one producer writes for one repo and one modality. A run
// replaces its shard: a row whose hash is unchanged keeps its vector, a changed
// or new row is embedded and upserted, and a row the run no longer produces is
// deleted, all in one transaction. Embedding happens before the transaction
// opens, so minutes of embed calls never hold a transaction open.
//
// Ids are unique within a store. nym keeps its unprefixed ids; every other
// producer sets `repo` on chunkCodeFile so its ids cannot collide with nym's.

import { chunkHash, embedChunks, type EmbedProvider, type Embeddable } from './embed.ts';

/**
 * One database connection: a pg `Client`, a client checked out of a pg `Pool`,
 * or PGlite. Never a `Pool` itself: a pool may run BEGIN, the writes, and COMMIT
 * on different connections, which silently drops the one-transaction guarantee.
 */
export interface Connection {
  query<T = Record<string, unknown>>(text: string, params?: unknown[]): Promise<{ rows: T[] }>;
}

export type Modality = 'docs' | 'code';

export interface StoreChunk extends Embeddable {
  id: string;
  source: string;
  title: string;
  heading: string;
  url: string;
  lang?: string;
  tokensEst?: number;
  test?: true;
}

export interface ShardStats {
  total: number;
  embedded: number;
  unchanged: number;
  deleted: number;
}

// `embedding` has no fixed dimension, so a store's model can change; a typed
// column matters only if an ANN index is ever added (design decision 1).
export const SCHEMA = [
  'create extension if not exists vector',
  `create table if not exists shards (
    shard text primary key,
    repo text not null,
    modality text not null,
    model text not null,
    dim integer not null,
    source_rev text,
    last_run timestamptz not null
  )`,
  `create table if not exists chunks (
    id text primary key,
    shard text not null references shards (shard) on delete cascade,
    repo text not null,
    modality text not null,
    source text not null,
    title text not null,
    heading text not null,
    url text not null,
    text text not null,
    lang text,
    tokens_est integer,
    test boolean not null default false,
    content_hash text not null,
    embedding vector not null
  )`,
  'create index if not exists chunks_shard on chunks (shard)',
  'create index if not exists chunks_modality on chunks (modality)',
];

export async function ensureSchema(db: Connection): Promise<void> {
  for (const statement of SCHEMA) await db.query(statement);
}

// Rows per statement, so one run never sends the whole shard as one parameter.
const ROWS_PER_STATEMENT = 500;

const inBatches = <T>(xs: T[]): T[][] =>
  Array.from({ length: Math.ceil(xs.length / ROWS_PER_STATEMENT) }, (_, i) => xs.slice(i * ROWS_PER_STATEMENT, (i + 1) * ROWS_PER_STATEMENT));

const sameHashes = (a: Map<string, string>, b: Map<string, string>) => a.size === b.size && [...a].every(([id, h]) => b.get(id) === h);

async function shardHashes(db: Connection, shard: string): Promise<Map<string, string>> {
  const { rows } = await db.query<{ id: string; content_hash: string }>('select id, content_hash from chunks where shard = $1', [shard]);
  return new Map(rows.map((r) => [r.id, r.content_hash]));
}

/**
 * Replace the shard `<repo>/<modality>` with `chunks`. Throws, and writes
 * nothing, on an empty run, a duplicate id, an id another shard owns, a model or
 * dimension that differs from another shard of the same modality, or a shard
 * that another run changed while this one was embedding. `sourceRev` is the
 * source commit, the model-independent freshness signal.
 */
export async function indexShard(
  db: Connection,
  { repo, modality, chunks, provider, sourceRev = null, concurrency }: {
    repo: string;
    modality: Modality;
    chunks: StoreChunk[];
    provider: EmbedProvider;
    sourceRev?: string | null;
    concurrency?: number;
  },
): Promise<ShardStats> {
  const shard = `${repo}/${modality}`;
  // With no ids the stale delete would empty the shard.
  if (chunks.length === 0) throw new Error(`shard ${shard}: the run produced no chunks; refusing to empty the shard`);
  const ids = chunks.map((c) => c.id);
  if (new Set(ids).size !== ids.length) throw new Error(`shard ${shard}: the run produced duplicate chunk ids`);

  const before = await shardHashes(db, shard);
  const hashes = new Map(chunks.map((c) => [c.id, chunkHash(c, provider)]));
  const changed = chunks.filter((c) => before.get(c.id) !== hashes.get(c.id));
  const unchanged = chunks.filter((c) => before.get(c.id) === hashes.get(c.id));
  const { embedded } = await embedChunks(changed, provider, new Map(), { concurrency });

  await db.query('begin');
  try {
    await db.query('select pg_advisory_xact_lock(hashtext($1))', [shard]);

    if (!sameHashes(before, await shardHashes(db, shard))) {
      throw new Error(`shard ${shard}: another run changed it while this one was embedding; run again`);
    }
    const mismatched = await db.query<{ shard: string; model: string; dim: number }>(
      'select shard, model, dim from shards where modality = $1 and shard <> $2 and (model <> $3 or dim <> $4)',
      [modality, shard, provider.model, provider.dim],
    );
    if (mismatched.rows.length) {
      const others = mismatched.rows.map((r) => `${r.shard} (${r.model}, ${r.dim})`).join(', ');
      throw new Error(`shard ${shard}: ${provider.model} (${provider.dim}) differs from the ${modality} model of ${others}; re-embed every ${modality} shard together`);
    }
    const taken = await db.query<{ id: string; shard: string }>('select id, shard from chunks where id = any($1::text[]) and shard <> $2', [ids, shard]);
    if (taken.rows.length) {
      const sample = taken.rows.slice(0, 5).map((r) => `${r.id} (owned by ${r.shard})`).join(', ');
      throw new Error(`shard ${shard}: ${taken.rows.length} chunk id(s) belong to another shard, e.g. ${sample}; set a repo prefix on the ids`);
    }

    await db.query(
      `insert into shards (shard, repo, modality, model, dim, source_rev, last_run) values ($1, $2, $3, $4, $5, $6, now())
       on conflict (shard) do update set model = excluded.model, dim = excluded.dim, source_rev = excluded.source_rev, last_run = excluded.last_run`,
      [shard, repo, modality, provider.model, provider.dim, sourceRev],
    );

    for (const batch of inBatches(embedded)) {
      await db.query(
        `insert into chunks (id, shard, repo, modality, source, title, heading, url, text, lang, tokens_est, test, content_hash, embedding)
         select v.id, $1, $2, $3, v.source, v.title, v.heading, v.url, v.text, v.lang, v.tokens_est, v.test, v.content_hash, v.embedding::vector
         from unnest($4::text[], $5::text[], $6::text[], $7::text[], $8::text[], $9::text[], $10::text[], $11::int[], $12::bool[], $13::text[], $14::text[])
           as v(id, source, title, heading, url, text, lang, tokens_est, test, content_hash, embedding)
         on conflict (id) do update set
           source = excluded.source, title = excluded.title, heading = excluded.heading, url = excluded.url, text = excluded.text,
           lang = excluded.lang, tokens_est = excluded.tokens_est, test = excluded.test,
           content_hash = excluded.content_hash, embedding = excluded.embedding`,
        [
          shard, repo, modality,
          batch.map((c) => c.id), batch.map((c) => c.source), batch.map((c) => c.title), batch.map((c) => c.heading),
          batch.map((c) => c.url), batch.map((c) => c.text), batch.map((c) => c.lang ?? null), batch.map((c) => c.tokensEst ?? null),
          batch.map((c) => c.test === true), batch.map((c) => hashes.get(c.id)), batch.map((c) => `[${c.vector.join(',')}]`),
        ],
      );
    }

    // The hash covers only the embedded text, so an unchanged row can still have
    // moved: a code chunk shifted down a file keeps its text and gets a new `#L` url.
    for (const batch of inBatches(unchanged)) {
      await db.query(
        `update chunks c set source = v.source, title = v.title, heading = v.heading, url = v.url,
           lang = v.lang, tokens_est = v.tokens_est, test = v.test
         from unnest($2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::int[], $9::bool[])
           as v(id, source, title, heading, url, lang, tokens_est, test)
         where c.id = v.id and c.shard = $1`,
        [
          shard,
          batch.map((c) => c.id), batch.map((c) => c.source), batch.map((c) => c.title), batch.map((c) => c.heading),
          batch.map((c) => c.url), batch.map((c) => c.lang ?? null), batch.map((c) => c.tokensEst ?? null), batch.map((c) => c.test === true),
        ],
      );
    }

    const deleted = await db.query('delete from chunks where shard = $1 and id <> all($2::text[]) returning id', [shard, ids]);
    await db.query('commit');
    return { total: chunks.length, embedded: embedded.length, unchanged: unchanged.length, deleted: deleted.rows.length };
  } catch (error) {
    await db.query('rollback');
    throw error;
  }
}
