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
    } catch (error) {
      setMixnodes({ data: null, error });
    }
  };

  const fetchGateways = async () => {
    try {
      const res = await Api.fetchGateways();
      setGateways({ data: res, error: null });
    } catch (error) {
      setGateways({ data: null, error });
    }
  };

  const fetchValidators = async () => {
    try {
      const res = await Api.fetchValidators();
      setValidators({ data: res, error: null });
    } catch (error) {
      setValidators({ data: null, error });
    }
  };
  const fetchBlock = async () => {
    try {
      const res = await Api.fetchBlock();
      setBlock({ data: res, error: null });
    } catch (error) {
      setBlock({ data: null, error });
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
