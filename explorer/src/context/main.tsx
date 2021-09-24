import { PaletteMode } from '@mui/material';
import * as React from 'react';
import { CountryDataResponse, GatewayResponse, MixNodeResponse, ValidatorsResponse, BlockResponse, ApiState } from 'src/typeDefs/explorer-api';
import { Api } from '../api';
interface State {
  mode: PaletteMode
  toggleMode?: () => void
  mixnodes?: ApiState<MixNodeResponse>
  gateways?: ApiState<GatewayResponse>
  validators?: ApiState<ValidatorsResponse>
  block?: ApiState<BlockResponse>
  countryData?: ApiState<CountryDataResponse>
};

export const MainContext = React.createContext<State>({ mode: "dark" });

export const MainContextProvider: React.FC = ({ children }) => {
  // light/dark mode
  const [mode, setMode] = React.useState<PaletteMode>('dark');

  // various APIs for cards on Overview
  const [mixnodes, setMixnodes] = React.useState<ApiState<MixNodeResponse>>();
  const [gateways, setGateways] = React.useState<ApiState<GatewayResponse>>();
  const [validators, setValidators] = React.useState<ApiState<ValidatorsResponse>>();
  const [block, setBlock] = React.useState<ApiState<BlockResponse>>();
  const [countryData, setCountryData] = React.useState<ApiState<CountryDataResponse>>();

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  const fetchMixnodes = async () => {
    try {
      const data = await Api.fetchMixnodes();
      setMixnodes({ data, isLoading: false })
    } catch (error) {
      setMixnodes({
        error: error instanceof Error ? error : new Error("Mixnode api fail"),
        isLoading: false
      });
    }
  };

  const fetchGateways = async () => {
    try {
      const data = await Api.fetchGateways();
      setGateways({ data, isLoading: false })
    } catch (error) {
      setGateways({
        error: error instanceof Error ? error : new Error("Gateways api fail"),
        isLoading: false
      });
    }
  };

  const fetchValidators = async () => {
    try {
      const data = await Api.fetchValidators();
      setValidators({ data, isLoading: false });
    } catch (error) {
      setValidators({
        error: error instanceof Error ? error : new Error("Validators api fail"),
        isLoading: false
      });
    }
  };

  const fetchBlock = async () => {
    try {
      const data = await Api.fetchBlock();
      setBlock({ data, isLoading: false });
    } catch (error) {
      setBlock({ 
        error: error instanceof Error ? error : new Error("Block api fail"),
        isLoading: false
      });
    }
  };

  const fetchCountryData = async () => {
    try {
      const res = await Api.fetchCountryData();
      setCountryData({ data: res, isLoading: false });
    } catch (error) {
      setCountryData({ 
        error: error instanceof Error ? error : new Error("Block api fail"),
        isLoading: false
      });
    }
  }

  React.useEffect(() => {
    Promise.all([
      fetchMixnodes(),
      fetchGateways(),
      fetchValidators(),
      fetchBlock(),
      fetchCountryData(),
    ])
  }, []);

  return (
    <MainContext.Provider
      value={{ mode, toggleMode, mixnodes, gateways, validators, block, countryData }}
    >
      {children}
    </MainContext.Provider>
  );
};
