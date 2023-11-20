import { AppState } from '../types';

export type StateAction =
  | { type: 'connect' }
  | { type: 'disconnect' }
  | { type: 'reset' };

export const initialState: AppState = {
  state: 'Disconnected',
  privacyMode: 'High',
  tunnel: { name: 'nym', id: 'nym' },
};

export function reducer(state: AppState, action: StateAction): AppState {
  switch (action.type) {
    case 'connect': {
      return { ...state, state: 'Connecting' };
    }
    case 'disconnect': {
      return { ...state, state: 'Disconnecting' };
    }
    case 'reset':
      return initialState;
  }
}
