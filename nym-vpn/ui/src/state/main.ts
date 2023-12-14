import dayjs from 'dayjs';
import {
  AppState,
  ConnectProgressMsg,
  ConnectionState,
  Country,
  NodeHop,
  UiTheme,
  VpnMode,
} from '../types';

export type StateAction =
  | { type: 'set-partial-state'; partialState: Partial<AppState> }
  | { type: 'change-connection-state'; state: ConnectionState }
  | { type: 'set-vpn-mode'; mode: VpnMode }
  | { type: 'set-error'; error: string }
  | { type: 'reset-error' }
  | { type: 'new-progress-message'; message: ConnectProgressMsg }
  | { type: 'connect' }
  | { type: 'disconnect' }
  | { type: 'set-connected'; startTime: number }
  | { type: 'set-connection-start-time'; startTime?: number | null }
  | { type: 'set-disconnected' }
  | { type: 'reset' }
  | { type: 'set-ui-theme'; theme: UiTheme }
  | { type: 'set-node-location'; payload: { hop: NodeHop; country: Country } };

export const initialState: AppState = {
  state: 'Disconnected',
  loading: false,
  vpnMode: 'Mixnet',
  tunnel: { name: 'nym', id: 'nym' },
  uiTheme: 'Light',
  progressMessages: [],
  entryNodeLocation: null,
  exitNodeLocation: null,
};

export function reducer(state: AppState, action: StateAction): AppState {
  switch (action.type) {
    case 'set-node-location':
      if (action.payload.hop === 'entry') {
        return {
          ...state,
          entryNodeLocation: action.payload.country,
        };
      }
      return {
        ...state,
        exitNodeLocation: action.payload.country,
      };
    case 'set-vpn-mode':
      return {
        ...state,
        vpnMode: action.mode,
      };
    case 'set-partial-state': {
      return { ...state, ...action.partialState };
    }
    case 'change-connection-state': {
      console.log(
        `__REDUCER [change-connection-state] changing connection state to ${action.state}`,
      );
      if (action.state === state.state) {
        return state;
      }
      return {
        ...state,
        state: action.state,
        loading:
          action.state === 'Connecting' || action.state === 'Disconnecting',
      };
    }
    case 'connect': {
      console.log(
        `__REDUCER [connect] changing connection state to Connecting`,
      );
      return { ...state, state: 'Connecting', loading: true };
    }
    case 'disconnect': {
      return { ...state, state: 'Disconnecting', loading: true };
    }
    case 'set-connected': {
      console.log(
        `__REDUCER [set-connected] changing connection state to Connected`,
      );
      return {
        ...state,
        state: 'Connected',
        loading: false,
        progressMessages: [],
        sessionStartDate: dayjs.unix(action.startTime),
      };
    }
    case 'set-disconnected': {
      return {
        ...state,
        state: 'Disconnected',
        loading: false,
        progressMessages: [],
        sessionStartDate: null,
      };
    }
    case 'set-connection-start-time':
      return {
        ...state,
        sessionStartDate:
          (action.startTime && dayjs.unix(action.startTime)) || null,
      };
    case 'set-error':
      return { ...state, error: action.error };
    case 'reset-error':
      return { ...state, error: null };
    case 'new-progress-message':
      return {
        ...state,
        progressMessages: [...state.progressMessages, action.message],
      };
    case 'set-ui-theme':
      return {
        ...state,
        uiTheme: action.theme,
      };
    case 'reset':
      return initialState;
  }
}
