import { useEffect, useReducer } from 'react';
import { invoke } from '@tauri-apps/api';
import { initialState, reducer } from './main';
import { useTauriEvents } from './useTauriEvents';
import { MainDispatchContext, MainStateContext } from '../contexts';
import { ConnectionState } from '../types';

type Props = {
  children?: React.ReactNode;
};

export function MainStateProvider({ children }: Props) {
  const [state, dispatch] = useReducer(reducer, initialState);

  useTauriEvents(dispatch);

  // initialize connection state
  useEffect(() => {
    const getInitialConnectionState = async () => {
      return await invoke<ConnectionState>('get_connection_state');
    };

    getInitialConnectionState().then((state) =>
      dispatch({ type: 'change-connection-state', state }),
    );
  }, []);

  return (
    <MainStateContext.Provider value={state}>
      <MainDispatchContext.Provider value={dispatch}>
        {children}
      </MainDispatchContext.Provider>
    </MainStateContext.Provider>
  );
}
