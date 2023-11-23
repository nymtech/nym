import React, { useEffect, useReducer } from 'react';
import { invoke } from '@tauri-apps/api';
import { initialState, reducer } from './main';
import { useTauriEvents } from './useTauriEvents';
import { MainDispatchContext, MainStateContext } from '../contexts';
import { AppDataFromBackend, CmdError, ConnectionState } from '../types';

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
      return await invoke<AppDataFromBackend>('get_app_data');
    };

    getAppData()
      .then((data) => {
        console.log(data);
        dispatch({
          type: 'set-app-data',
          data: {
            autoconnect: data.autoconnect || false,
            monitoring: data.monitoring || false,
            killswitch: data.killswitch || false,
            uiMode: data.ui_mode || 'Light',
            privacyMode: data.privacy_mode || 'High',
            entryNode: data.entry_node,
            exitNode: data.exit_node,
          },
        });
        dispatch({
          type: 'set-partial-state',
          partialState: {
            uiMode: data.ui_mode || 'Light',
            privacyMode: data.privacy_mode || 'High',
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
