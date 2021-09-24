import { PaletteMode } from '@mui/material';
import * as React from 'react';
import { CountryDataResponse, GatewayResponse, MixNodeResponse, ValidatorsResponse, BlockResponse } from 'src/typeDefs/explorer-api';
import { Api } from '../api';

// type NodeApiResponse = {
//   data: MixNodeResponse | GatewayResponse | ValidatorsResponse | CountryDataResponse | number | null
//   error: string | null
// }

interface ApiState<RESPONSE> {
  isLoading: boolean;
  data?: RESPONSE;
  error?: Error;
}
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

      let errorMessage = '';

      if (error instanceof Error) {
        errorMessage = error.message
      }
      setMixnodes({ error: errorMessage, isLoading: false });
    }
  };
  const fetchGateways = async () => {
    try {
      const res = await Api.fetchGateways();
      setGateways({ data: res, error: null });
    } catch (error) {
      setGateways({ data: null, error: error instanceof Error ? error.message : '' });
    }
  };
  const fetchValidators = async () => {
    try {
      const res = await Api.fetchValidators();
      setValidators({ data: res, error: null });
    } catch (error) {
      setValidators({ data: null, error: error instanceof Error ? error.message : '' });
    }
  };
  const fetchBlock = async () => {
    try {
      const res = await Api.fetchBlock();
      setBlock({ data: res, error: null });
    } catch (error) {
      setBlock({ data: null, error: error instanceof Error ? error.message : '' });
    }
  };
  const fetchCountryData = async () => {
    try {
      const res = await Api.fetchCountryData();
      setCountryData({ data: res, error: null });
    } catch (error) {
      setCountryData({ data: null, error: error instanceof Error ? error.message : '' });
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
