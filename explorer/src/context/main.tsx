import * as React from 'react';
import { Api } from '../api';

export const MainContext = React.createContext({});

export const MainContextProvider: React.FC = ({ children }: any) => {
  // light/dark mode
  const [mode, setMode] = React.useState('light');

  // various APIs for cards on Overview
  const [mixnodes, setMixnodes] = React.useState<any>();
  const [gateways, setGateways] = React.useState<any>();
  const [validators, setValidators] = React.useState<any>();
  const [block, setBlock] = React.useState<any>();

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  const fetchMixnodes = async () => {
    try {
      const res = await Api.fetchMixnodes();
      setMixnodes({ data: res, error: null });
    } catch (error: any) {
      setMixnodes({ data: null, error: error.message });
    }
  };

  const fetchGateways = async () => {
    try {
      const res = await Api.fetchGateways();
      setGateways({ data: res, error: null });
    } catch (error:any) {
      setGateways({ data: null, error: error.message });
    }
  };

  const fetchValidators = async () => {
    try {
      const res = await Api.fetchValidators();
      setValidators({ data: res, error: null });
    } catch (error:any) {
      setValidators({ data: null, error: error.message });
    }
  };
  const fetchBlock = async () => {
    try {
      const res = await Api.fetchBlock();
      setBlock({ data: res, error: null });
    } catch (error:any) {
      setBlock({ data: null, error: error.message });
    }
  };

  React.useEffect(() => {
    fetchMixnodes();
    fetchGateways();
    fetchValidators();
    fetchBlock();


  }, []);

  return (
    <MainContext.Provider
      value={{ mode, toggleMode, mixnodes, gateways, validators, block }}
    >
      {children}
    </MainContext.Provider>
  );
};
