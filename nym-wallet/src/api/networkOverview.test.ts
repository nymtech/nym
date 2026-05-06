import { fetchNymPriceDeduped } from './networkOverview';

const sampleTokenomics = {
  quotes: {
    USD: {
      price: 0.0331,
      market_cap: 26_000_000,
      volume_24h: 1_200_000,
    },
  },
};

describe('fetchNymPriceDeduped', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('coalesces concurrent requests for the same URL', async () => {
    let callCount = 0;
    global.fetch = jest.fn(() => {
      callCount += 1;
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve(sampleTokenomics),
      } as Response);
    });

    const url = 'https://api.example.test/v1/nym-price';
    const p1 = fetchNymPriceDeduped(url);
    const p2 = fetchNymPriceDeduped(url);
    const [a, b] = await Promise.all([p1, p2]);

    expect(a).toStrictEqual(sampleTokenomics);
    expect(b).toStrictEqual(sampleTokenomics);
    expect(callCount).toBe(1);
  });

  it('does not coalesce different URLs', async () => {
    let callCount = 0;
    global.fetch = jest.fn(() => {
      callCount += 1;
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve(sampleTokenomics),
      } as Response);
    });

    await Promise.all([fetchNymPriceDeduped('https://a.test/p'), fetchNymPriceDeduped('https://b.test/p')]);
    expect(callCount).toBe(2);
  });
});
