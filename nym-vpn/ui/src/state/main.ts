import { AppState, ConnectionState } from '../types';

export type StateAction =
  | { type: 'change-connection-state'; state: ConnectionState }
  | { type: 'connect' }
  | { type: 'disconnect' }
  | { type: 'reset' };

export const initialState: AppState = {
  state: 'Disconnected',
  loading: false,
  privacyMode: 'High',
  tunnel: { name: 'nym', id: 'nym' },
};

export function reducer(state: AppState, action: StateAction): AppState {
  switch (action.type) {
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
    case 'reset':
      return initialState;
  }
}
