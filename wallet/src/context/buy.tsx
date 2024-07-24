import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { sign } from '@src/requests';
import { Console } from '@src/utils/console';
// import { AppContext } from './main';

export type TBuyContext = {
  loading: boolean;
  error?: string;
  signMessage: (message: string) => Promise<string | undefined>;
  refresh: () => Promise<void>;
};

export const BuyContext = createContext<TBuyContext>({
  loading: false,
  signMessage: async () => '',
  refresh: async () => undefined,
});

export const BuyContextProvider: FCWithChildren = ({ children }): JSX.Element => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();

  const refresh = useCallback(async () => {
    setError(undefined);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const signMessage = async (message: string) => {
    let signature;
    setLoading(true);
    try {
      signature = await sign(message);
    } catch (e: any) {
      Console.log(`Sign message operation failed: ${e}`);
      setError(`Sign message operation failed: ${e}`);
    } finally {
      setLoading(false);
    }
    return signature;
  };

  const memoizedValue = useMemo(
    () => ({
      loading,
      error,
      refresh,
      signMessage,
    }),
    [loading, error],
  );

  return <BuyContext.Provider value={memoizedValue}>{children}</BuyContext.Provider>;
};

export const useBuyContext = () => useContext<TBuyContext>(BuyContext);
