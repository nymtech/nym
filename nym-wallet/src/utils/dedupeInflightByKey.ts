export function dedupeInflightByKey<K, T>(inflight: Map<K, Promise<T>>, key: K, load: () => Promise<T>): Promise<T> {
  const existing = inflight.get(key);
  if (existing) {
    return existing;
  }

  const pending = load().finally(() => {
    inflight.delete(key);
  });

  inflight.set(key, pending);
  return pending;
}
