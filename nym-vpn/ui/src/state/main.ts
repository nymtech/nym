import { AppData, AppState, ConnectionState } from '../types';

export type StateAction =
  | { type: 'set-partial-state'; partialState: Partial<AppState> }
  | { type: 'change-connection-state'; state: ConnectionState }
  | { type: 'connect' }
  | { type: 'disconnect' }
  | { type: 'reset' }
  | { type: 'set-app-data'; data: AppData };

export const initialState: AppState = {
  state: 'Disconnected',
  loading: false,
  privacyMode: 'High',
  tunnel: { name: 'nym', id: 'nym' },
  uiMode: 'Light',
  localAppData: {
    monitoring: false,
    autoconnect: false,
    killswitch: false,
    uiMode: 'Light',
    privacyMode: 'High',
    entryNode: null,
    exitNode: null,
  },
};

export function reducer(state: AppState, action: StateAction): AppState {
  switch (action.type) {
    case 'set-partial-state': {
      return { ...state, ...action.partialState };
    }
    case 'change-connection-state': {
      return {
        ...state,
        state: action.state,
        loading:
          action.state === 'Connecting' || action.state === 'Disconnecting',
      };
    }
    case 'connect': {
      return { ...state, state: 'Connecting', loading: true };
    }
    case 'disconnect': {
      return { ...state, state: 'Disconnecting', loading: true };
    }
    case 'set-app-data': {
      return { ...state, localAppData: action.data };
    }
    case 'reset':
      return initialState;
  }
}
