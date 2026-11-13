// nym deploy policy for the Voyage key, kept out of @nymproject/retrieval-indexer.
export function resolveEmbedKey(artifact: string): string | undefined {
  const apiKey = process.env.VOYAGE_API_KEY;
  if (apiKey) return apiKey;
  // Vercel sets VERCEL on every build it serves. REQUIRE_EMBEDDINGS is the opt-in
  // for deploy pipelines elsewhere, so a workflow that publishes cannot quietly
  // lose its key. Plain CI is deliberately not enough.
  if (process.env.VERCEL || process.env.REQUIRE_EMBEDDINGS) {
    throw new Error(
      `VOYAGE_API_KEY is not set. Refusing to build ${artifact} without vectors: ` +
        `it would deploy cleanly and then return nothing for every query. ` +
        `Set VOYAGE_API_KEY in the build and runtime environment.`,
    );
  }
  console.warn(
    `VOYAGE_API_KEY not set: writing ${artifact} without vectors. ` +
      `Retrieval will return nothing until you rebuild with a key.`,
  );
  return undefined;
}
