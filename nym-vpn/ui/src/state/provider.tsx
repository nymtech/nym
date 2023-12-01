import React, { useEffect, useReducer } from 'react';
import { invoke } from '@tauri-apps/api';
import { MainDispatchContext, MainStateContext } from '../contexts';
import { AppDataFromBackend, CmdError, ConnectionState } from '../types';
import { initialState, reducer } from './main';
import { useTauriEvents } from './useTauriEvents';

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

    // initialize session start time
    const getSessionStartTime = async () => {
      return await invoke<number | undefined>('get_connection_start_time');
    };

    getInitialConnectionState().then((state) =>
      dispatch({ type: 'change-connection-state', state }),
    );
    getSessionStartTime().then((startTime) =>
      dispatch({ type: 'set-connection-start-time', startTime }),
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
            uiTheme: data.ui_theme || 'Light',
            vpnMode: data.vpn_mode || 'TwoHop',
            entryNode: data.entry_node,
            exitNode: data.exit_node,
          },
        });
        dispatch({
          type: 'set-partial-state',
          partialState: {
            uiTheme: data.ui_theme || 'Light',
            vpnMode: data.vpn_mode || 'TwoHop',
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
