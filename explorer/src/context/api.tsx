import * as React from 'react';
import { createContext } from 'react';
import { Data } from '../api';

export const ApiDataContext = createContext({});

export const ApiDataProvider: React.FC = ({ children }: any) => {
  const [data, setData] = React.useState<any>();

  const fetchAllData = async () => {
    try {
      const mixnodes = await Data.fetchMixnodes();
      const gateways = await Data.fetchGateways();
      const validators = await Data.fetchValidators();
      setData({
        mixnodes,
        gateways,
        validators,
      });
    } catch (error) {
      console.log('error ', error);
    }
  };
  React.useEffect(() => {
    fetchAllData();
  }, []);
  return (
    <ApiDataContext.Provider value={data}>{children}</ApiDataContext.Provider>
  );
};
