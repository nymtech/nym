import { useEffect, useMemo, useState } from 'react';
import { fetchNymPriceDeduped, getNetworkOverviewEndpoints } from 'src/api/networkOverview';
import type { Network } from 'src/types';

export type UseNymUsdPrice = {
  usdPerNym: number | undefined;
  loading: boolean;
  error: string | undefined;
};

export function useNymUsdPrice(network: Network | undefined): UseNymUsdPrice {
  const [usdPerNym, setUsdPerNym] = useState<number | undefined>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  const url = useMemo(() => {
    if (network === undefined) {
      return undefined;
    }
    return getNetworkOverviewEndpoints(network).nymPrice;
  }, [network]);

  useEffect(() => {
    if (!url) {
      setUsdPerNym(undefined);
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
