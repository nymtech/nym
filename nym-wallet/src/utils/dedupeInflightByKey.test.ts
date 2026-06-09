import { dedupeInflightByKey } from './dedupeInflightByKey';

describe('dedupeInflightByKey', () => {
  it('reuses the in-flight promise for the same key', async () => {
    const inflight = new Map<string, Promise<string>>();
    let calls = 0;
    const load = () => {
      calls += 1;
      return new Promise<string>((resolve) => {
        setTimeout(() => resolve('done'), 20);
      });
    };

    const first = dedupeInflightByKey(inflight, 'MAINNET', load);
    const second = dedupeInflightByKey(inflight, 'MAINNET', load);

    expect(first).toBe(second);
    await first;
    expect(calls).toBe(1);
  });

  it('does not reuse promises across different keys', async () => {
    const inflight = new Map<string, Promise<string>>();
    let calls = 0;
    const load = (value: string) => () => {
      calls += 1;
      return Promise.resolve(value);
    };

    const mainnet = dedupeInflightByKey(inflight, 'MAINNET', load('mainnet'));
    const sandbox = dedupeInflightByKey(inflight, 'SANDBOX', load('sandbox'));

    await expect(mainnet).resolves.toBe('mainnet');
    await expect(sandbox).resolves.toBe('sandbox');
    expect(calls).toBe(2);
  });

  it('clears the key after the promise settles', async () => {
    const inflight = new Map<string, Promise<string>>();
    await dedupeInflightByKey(inflight, 'MAINNET', async () => 'done');
    expect(inflight.has('MAINNET')).toBe(false);
  });
});
