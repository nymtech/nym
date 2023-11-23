import { useEffect, useReducer } from 'react';
import { invoke } from '@tauri-apps/api';
import { initialState, reducer } from './main';
import { useTauriEvents } from './useTauriEvents';
import { MainDispatchContext, MainStateContext } from '../contexts';
import { AppDataFromStorage, CmdError, ConnectionState } from '../types';

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

  // get saved on disk app data and restore state from it
  useEffect(() => {
    const getAppData = async () => {
      return await invoke<AppDataFromStorage>('get_app_data');
    };

    getAppData()
      .then((state) => {
        dispatch({
          type: 'set-app-data',
          data: {
            autoconnect: state.autoconnect || false,
            monitoring: state.monitoring || false,
            killswitch: state.killswitch || false,
            uiMode: state.uiMode || 'Light',
            privacyMode: state.privacyMode || 'High',
            entryNode: state.entryNode,
            exitNode: state.exitNode,
          },
        });
        dispatch({
          type: 'set-partial-state',
          partialState: {
            uiMode: state.uiMode || 'Light',
            privacyMode: state.privacyMode || 'High',
          },
        });
      })
      .catch((err: CmdError) => {
        // TODO handle error properly
        console.log(err);
      });
  }, []);

  return (
    <MainStateContext.Provider value={state}>
      <MainDispatchContext.Provider value={dispatch}>
        {children}
      </MainDispatchContext.Provider>
    </MainStateContext.Provider>
  );
}
