import { PaletteMode } from '@mui/material';
import * as React from 'react';
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
} from 'src/typeDefs/explorer-api';
import { Api } from '../api';

interface State {
  mode: PaletteMode;
  toggleMode: () => void;
  mixnodes?: ApiState<MixNodeResponse>;
  fetchMixnodes: () => void;
  filterMixnodes: (arg: MixNodeResponse) => void;
  gateways?: ApiState<GatewayResponse>;
  validators?: ApiState<ValidatorsResponse>;
  block?: ApiState<BlockResponse>;
  countryData?: ApiState<CountryDataResponse>;
  globalError?: string | undefined;
  mixnodeDetailInfo?: ApiState<MixNodeResponse>;
  fetchMixnodeById: (arg: string) => void;
  fetchDelegationsById: (arg: string) => void;
  delegations?: ApiState<DelegationsResponse>;
  stats?: ApiState<StatsResponse>;
  fetchStatsById: (arg: string) => void;
  status?: ApiState<StatusResponse>;
  fetchStatusById: (arg: string) => void;
  fetchUptimeStoryById: (arg: string) => void;
  uptimeStory?: ApiState<UptimeStoryResponse>;
}

export const MainContext = React.createContext<State>({
  mode: 'dark',
  fetchMixnodeById: () => null,
  toggleMode: () => undefined,
  fetchDelegationsById: () => null,
  fetchStatsById: () => null,
  fetchStatusById: () => null,
  fetchUptimeStoryById: () => null,
  filterMixnodes: () => null,
  fetchMixnodes: () => null,
  status: { data: undefined, isLoading: false, error: undefined },
  stats: { data: undefined, isLoading: false, error: undefined },
  mixnodeDetailInfo: { data: undefined, isLoading: false, error: undefined },
  delegations: { data: undefined, isLoading: false, error: undefined },
});

export const MainContextProvider: React.FC = ({ children }) => {
  // light/dark mode
  const [mode, setMode] = React.useState<PaletteMode>('dark');

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
  const [mixnodeDetailInfo, setMixnodeDetailInfo] =
    React.useState<ApiState<MixNodeResponse>>();

  // various APIs for Detail page
  const [delegations, setDelegations] =
    React.useState<ApiState<DelegationsResponse>>();
  const [status, setStatus] = React.useState<ApiState<StatusResponse>>();
  const [stats, setStats] = React.useState<ApiState<StatsResponse>>();
  const [uptimeStory, setUptimeStory] =
    React.useState<ApiState<UptimeStoryResponse>>();

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  // actions passed down to Overview and Detail pages
  const fetchUptimeStoryById = async (id: string) => {
    setUptimeStory({
      data: uptimeStory?.data,
      isLoading: true,
      error: uptimeStory?.error,
    });
    try {
      const data = await Api.fetchUptimeStoryById(id);
      setUptimeStory({ data, isLoading: false });
    } catch (error) {
      setUptimeStory({
        error:
          error instanceof Error ? error : new Error('Uptime Story api fail'),
        isLoading: false,
      });
    }
  };

  const fetchDelegationsById = async (id: string) => {
    setDelegations({ data: delegations?.data, isLoading: true });
    try {
      const data = await Api.fetchDelegationsById(id);
      setDelegations({ data, isLoading: false });
    } catch (error) {
      setDelegations({
        error:
          error instanceof Error ? error : new Error('Delegations api fail'),
        isLoading: false,
      });
    }
  };

  const fetchStatusById = async (id: string) => {
    setStatus({ data: status?.data, isLoading: true, error: status?.error });
    try {
      const data = await Api.fetchStatusById(id);
      setStatus({ data, isLoading: false });
    } catch (error) {
      setStatus({
        error: error instanceof Error ? error : new Error('Status api fail'),
        isLoading: false,
      });
    }
  };

  const fetchStatsById = async (id: string) => {
    setStats({ data: stats?.data, isLoading: true, error: stats?.error });
    try {
      const data = await Api.fetchStatsById(id);
      setStats({ data, isLoading: false });
    } catch (error) {
      setStats({
        error: error instanceof Error ? error : new Error('Stats api fail'),
        isLoading: false,
      });
    }
  };

  const fetchMixnodeById = async (id: string) => {
    setMixnodeDetailInfo({ data: mixnodeDetailInfo?.data, isLoading: true });

    // 1. if mixnode data already exists filter down to this ID
    if (mixnodes && mixnodes.data) {
      const [matchedToID] = mixnodes.data.filter(
        (eachMixnode: MixNodeResponseItem) =>
          eachMixnode.mix_node.identity_key === id,
      );

      // b) SUCCESS | if there *IS* a matched ID in mixnodes
      if (matchedToID) {
        setMixnodeDetailInfo({ data: [matchedToID], isLoading: false });
      }
      // b) FAIL | if there is no matching ID in mixnodes
      if (!matchedToID) {
        setGlobalError(MIXNODE_API_ERROR);
        setMixnodeDetailInfo({
          isLoading: false,
          error: new Error(MIXNODE_API_ERROR),
        });
      }
    } else {
      // 2. if mixnode data DOESNT already exist, fetch this specific ID's information.
      try {
        const data = await Api.fetchMixnodeByID(id);
        // a) fetches from cache^, then API, then filters down then dumps in `mixnodes` context.
        if (data) {
          setMixnodeDetailInfo({ data: [data], isLoading: false });
        } else {
          throw Error('api failed to retrieve mixnode via id');
        }
        // NOTE: Only returning mixnodes api info at the moment. Other `ping` api required also.
      } catch (error) {
        setGlobalError(MIXNODE_API_ERROR);
        setMixnodeDetailInfo({
          isLoading: false,
          error: new Error(MIXNODE_API_ERROR),
        });
      }
    }
  };
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
        mode,
        toggleMode,
        mixnodes,
        filterMixnodes,
        fetchMixnodes,
        gateways,
        validators,
        block,
        countryData,
        globalError,
        mixnodeDetailInfo,
        fetchMixnodeById,
        fetchDelegationsById,
        delegations,
        stats,
        fetchStatsById,
        status,
        fetchStatusById,
        uptimeStory,
        fetchUptimeStoryById,
      }}
    >
      {children}
    </MainContext.Provider>
  );
};
