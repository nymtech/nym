import { useEffect, useState } from 'react';

/** Unix seconds that tick every second — used for invitation TTL display. */
export const useNowSecs = (): number => {
  const [nowSecs, setNowSecs] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    const id = window.setInterval(() => setNowSecs(Math.floor(Date.now() / 1000)), 1000);
    return () => window.clearInterval(id);
  }, []);

  return nowSecs;
};
