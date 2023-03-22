import * as React from 'react';
import { PaletteMode } from '@mui/material';
import {
  ApiState,
  BlockResponse,
  CountryDataResponse,
  DirectoryService,
  GatewayResponse,
  MixNodeResponse,
  MixnodeStatus,
  SummaryOverviewResponse,
  ValidatorsResponse,
  Environment,
} from '../typeDefs/explorer-api';
import { EnumFilterKey } from '../typeDefs/filters';
import { Api, getEnvironment } from '../api';
import { NavOptionType, originalNavOptions } from './nav';

interface StateData {
  summaryOverview?: ApiState<SummaryOverviewResponse>;
  block?: ApiState<BlockResponse>;
  countryData?: ApiState<CountryDataResponse>;
  gateways?: ApiState<GatewayResponse>;
  globalError?: string | undefined;
  mixnodes?: ApiState<MixNodeResponse>;
  mode: PaletteMode;
  navState: NavOptionType[];
  validators?: ApiState<ValidatorsResponse>;
  environment?: Environment;
  serviceProviders?: ApiState<DirectoryService>;
}

interface StateApi {
  fetchMixnodes: (status?: MixnodeStatus) => Promise<MixNodeResponse | undefined>;
  filterMixnodes: (filters: any, status: any) => void;
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
  fetchMixnodes: () => Promise.resolve(undefined),
});

export const useMainContext = (): React.ContextType<typeof MainContext> => React.useContext<State>(MainContext);

export const MainContextProvider: FCWithChildren = ({ children }) => {
  // network explorer environment
  const [environment, setEnvironment] = React.useState<Environment>('mainnet');

  // light/dark mode
  const [mode, setMode] = React.useState<PaletteMode>('dark');

  // nav state
  const [navState, updateNav] = React.useState<NavOptionType[]>(originalNavOptions);

  // global / banner error messaging
  const [globalError] = React.useState<string>();

  // various APIs for Overview page
  const [summaryOverview, setSummaryOverview] = React.useState<ApiState<SummaryOverviewResponse>>();
  const [mixnodes, setMixnodes] = React.useState<ApiState<MixNodeResponse>>();
  const [gateways, setGateways] = React.useState<ApiState<GatewayResponse>>();
  const [validators, setValidators] = React.useState<ApiState<ValidatorsResponse>>();
  const [block, setBlock] = React.useState<ApiState<BlockResponse>>();
  const [countryData, setCountryData] = React.useState<ApiState<CountryDataResponse>>();
  const [serviceProviders, setServiceProviders] = React.useState<ApiState<DirectoryService>>();

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  const fetchOverviewSummary = async () => {
    try {
      const data = await Api.fetchOverviewSummary();
      setSummaryOverview({ data, isLoading: false });
    } catch (error) {
      setSummaryOverview({
        error: error instanceof Error ? error : new Error('Overview summary api fail'),
        isLoading: false,
      });
    }
  };

  const fetchMixnodes = async (status?: MixnodeStatus) => {
    let data;
    setMixnodes((d) => ({ ...d, isLoading: true }));
    try {
      data = status ? await Api.fetchMixnodesActiveSetByStatus(status) : await Api.fetchMixnodes();
      setMixnodes({ data, isLoading: false });
    } catch (error) {
      setMixnodes({
        error: error instanceof Error ? error : new Error('Mixnode api fail'),
        isLoading: false,
      });
    }
    return data;
  };

  const filterMixnodes = async (filters: { [key in EnumFilterKey]: number[] }, status?: MixnodeStatus) => {
    setMixnodes((d) => ({ ...d, isLoading: true }));
    const mxns = status ? await Api.fetchMixnodesActiveSetByStatus(status) : await Api.fetchMixnodes();

    const filtered = mxns?.filter(
      (m) =>
        +m.profit_margin_percent >= filters.profitMargin[0] / 100 &&
        +m.profit_margin_percent <= filters.profitMargin[1] / 100 &&
        m.stake_saturation >= filters.stakeSaturation[0] &&
        m.stake_saturation <= filters.stakeSaturation[1] &&
        m.avg_uptime >= filters.routingScore[0] &&
        m.avg_uptime <= filters.routingScore[1],
    );

    setMixnodes({ data: filtered, isLoading: false });
  };

  const fetchGateways = async () => {
    setGateways((d) => ({ ...d, isLoading: true }));
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
        error: error instanceof Error ? error : new Error('Validators api fail'),
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
        error: error instanceof Error ? error : new Error('Country Data api fail'),
        isLoading: false,
      });
    }
  };

  const fetchServiceProviders = async () => {
    setServiceProviders({ data: undefined, isLoading: true });
    try {
      const [res] = await Api.fetchServiceProviders();
      setServiceProviders({ data: res, isLoading: false });
    } catch (error) {
      setServiceProviders({
        error: error instanceof Error ? error : new Error('Service provider api fail'),
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
      fetchOverviewSummary(),
      fetchGateways(),
      fetchValidators(),
      fetchBlock(),
      fetchCountryData(),
      fetchServiceProviders(),
    ]);
    setEnvironment(getEnvironment());
  }, []);

  const state = React.useMemo<State>(
    () => ({
      environment,
      block,
      countryData,
      fetchMixnodes,
      filterMixnodes,
      gateways,
      globalError,
      mixnodes,
      mode,
      navState,
      summaryOverview,
      toggleMode,
      updateNavState,
      validators,
      serviceProviders,
    }),
    [
      environment,
      block,
      countryData,
      gateways,
      globalError,
      mixnodes,
      mode,
      navState,
      summaryOverview,
      validators,
      serviceProviders,
    ],
  );

  return <MainContext.Provider value={state}>{children}</MainContext.Provider>;
};
