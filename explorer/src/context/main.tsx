import * as React from 'react';
import { PaletteMode } from '@mui/material';
import { MIXNODE_API_ERROR } from 'src/api/constants';
import {
  CountryDataResponse,
  GatewayResponse,
  MixNodeResponse,
  ValidatorsResponse,
  BlockResponse,
  ApiState,
  MixNodeResponseItem,
  DelegationsResponse,
  StatsResponse,
  StatusResponse,
  UptimeStoryResponse,
  MixNodeDescriptionResponse,
} from 'src/typeDefs/explorer-api';
import { Api } from '../api';
import { navOptionType, originalNavOptions } from './nav';

interface StateData {
  block?: ApiState<BlockResponse>;
  countryData?: ApiState<CountryDataResponse>;
  gateways?: ApiState<GatewayResponse>;
  globalError?: string | undefined;
  mixnodes?: ApiState<MixNodeResponse>;
  mode: PaletteMode;
  navState: navOptionType[];
  validators?: ApiState<ValidatorsResponse>;
}

interface StateApi {
  fetchMixnodes: () => void;
  filterMixnodes: (arg: MixNodeResponse) => void;
  toggleMode: () => void;
  updateNavState: (id: number) => void;
}

type State = StateData & StateApi;

export const MainContext = React.createContext<State>({
  mode: 'dark',
  updateNavState: () => null,
  navState: originalNavOptions,
  toggleMode: () => undefined,
  filterMixnodes: () => null,
  fetchMixnodes: () => null,
});

export const useMainContext = (): React.ContextType<typeof MainContext> =>
  React.useContext<State>(MainContext);

export const MainContextProvider: React.FC = ({ children }) => {
  // light/dark mode
  const [mode, setMode] = React.useState<PaletteMode>('dark');

  // nav state
  const [navState, updateNav] =
    React.useState<navOptionType[]>(originalNavOptions);

  // global / banner error messaging
  const [globalError, setGlobalError] = React.useState<string>();

  // various APIs for Overview page
  const [mixnodes, setMixnodes] = React.useState<ApiState<MixNodeResponse>>();
  const [gateways, setGateways] = React.useState<ApiState<GatewayResponse>>();
  const [validators, setValidators] =
    React.useState<ApiState<ValidatorsResponse>>();
  const [block, setBlock] = React.useState<ApiState<BlockResponse>>();
  const [countryData, setCountryData] =
    React.useState<ApiState<CountryDataResponse>>();

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  const fetchMixnodes = async () => {
    setMixnodes((d) => ({ ...d, isLoading: true }));
    try {
      const data = await Api.fetchMixnodes();
      setMixnodes({ data, isLoading: false });
    } catch (error) {
      setMixnodes({
        error: error instanceof Error ? error : new Error('Mixnode api fail'),
        isLoading: false,
      });
    }
  };
  const filterMixnodes = (arr: MixNodeResponse) => {
    setMixnodes({ data: arr, isLoading: false });
  };
  const fetchGateways = async () => {
    try {
      const data = await Api.fetchGateways();
      setGateways({ data, isLoading: false });
    } catch (error) {
      setGateways({
        error: error instanceof Error ? error : new Error('Gateways api fail'),
        isLoading: false,
      });
    }
  };
  const fetchValidators = async () => {
    try {
      const data = await Api.fetchValidators();
      setValidators({ data, isLoading: false });
    } catch (error) {
      setValidators({
        error:
          error instanceof Error ? error : new Error('Validators api fail'),
        isLoading: false,
      });
    }
  };
  const fetchBlock = async () => {
    try {
      const data = await Api.fetchBlock();
      setBlock({ data, isLoading: false });
    } catch (error) {
      setBlock({
        error: error instanceof Error ? error : new Error('Block api fail'),
        isLoading: false,
      });
    }
  };
  const fetchCountryData = async () => {
    setCountryData({ data: undefined, isLoading: true });
    try {
      const res = await Api.fetchCountryData();
      setCountryData({ data: res, isLoading: false });
    } catch (error) {
      setCountryData({
        error:
          error instanceof Error ? error : new Error('Country Data api fail'),
        isLoading: false,
      });
    }
  };
  const updateNavState = (id: number) => {
    const updated = navState.map((option) => ({
      ...option,
      isActive: option.id === id,
    }));
    updateNav(updated);
  };
  React.useEffect(() => {
    Promise.all([
      fetchMixnodes(),
      fetchGateways(),
      fetchValidators(),
      fetchBlock(),
      fetchCountryData(),
    ]);
  }, []);

  return (
    <MainContext.Provider
      value={{
        block,
        countryData,
        fetchMixnodes,
        filterMixnodes,
        gateways,
        globalError,
        mixnodes,
        mode,
        navState,
        toggleMode,
        updateNavState,
        validators,
      }}
    >
      {children}
    </MainContext.Provider>
  );
};
