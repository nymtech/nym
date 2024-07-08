import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { BuyContext } from '../buy';
import { mockSleep } from './utils';

export const MockBuyContextProvider: FCWithChildren = ({ children }): JSX.Element => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();

  const refresh = useCallback(async () => {
    setError(undefined);
  }, []);

  const signMessage = async (message: string) => {
    setLoading(true);
    await mockSleep(1042);
    setLoading(false);
    return `imagineareallyrealisticsignaturehash${message}`;
  };

  useEffect(() => {
    refresh();
  }, [refresh]);

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
