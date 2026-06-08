import { useEffect, useMemo, useState } from 'react';
import { fetchNymPriceDeduped, getCachedNymPrice, getNetworkOverviewEndpoints } from 'src/api/networkOverview';
import type { Network } from 'src/types';

export type UseNymUsdPrice = {
  usdPerNym: number | undefined;
  loading: boolean;
  error: string | undefined;
};

export function useNymUsdPrice(network: Network | undefined): UseNymUsdPrice {
  const url = useMemo(() => {
    if (network === undefined) {
      return undefined;
    }
    return getNetworkOverviewEndpoints(network).nymPrice;
  }, [network]);

  const cached = url ? getCachedNymPrice(url) : undefined;
  const [usdPerNym, setUsdPerNym] = useState<number | undefined>(cached?.quotes.USD.price);
  const [loading, setLoading] = useState(Boolean(url && !cached));
  const [error, setError] = useState<string | undefined>();

  useEffect(() => {
    if (!url) {
      setUsdPerNym(undefined);
      setLoading(false);
      setError(undefined);
      return undefined;
    }

    const cachedPrice = getCachedNymPrice(url);
    if (cachedPrice) {
      setUsdPerNym(cachedPrice.quotes.USD.price);
      setLoading(false);
      setError(undefined);
      return undefined;
    }

    let cancelled = false;
    setLoading(true);
    setError(undefined);

    fetchNymPriceDeduped(url)
      .then((data) => {
        if (!cancelled) {
          setUsdPerNym(data.quotes.USD.price);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setUsdPerNym(undefined);
          setError('Could not load NYM price');
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return (): void => {
      cancelled = true;
    };
  }, [url]);

  return { usdPerNym, loading, error };
}
