import React, { useEffect, useReducer } from 'react';
import { invoke } from '@tauri-apps/api';
import { getVersion } from '@tauri-apps/api/app';
import { MainDispatchContext, MainStateContext } from '../contexts';
import {
  AppDataFromBackend,
  CmdError,
  ConnectionState,
  Country,
} from '../types';
import { DefaultRootFontSize } from '../constants';
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

    // init country list
    const getCountries = async () => {
      return await invoke<Country[]>('get_node_countries');
    };

    // init default node location
    const getDefaultNodeLocation = async () => {
      return await invoke<Country>('get_default_node_location');
    };

    getVersion()
      .then((version) =>
        dispatch({
          type: 'set-version',
          version,
        }),
      )
      .catch((e) => {
        console.warn(`command [set-version] returned an error: ${e}`);
      });

    getInitialConnectionState()
      .then((state) => dispatch({ type: 'change-connection-state', state }))
      .catch((e: CmdError) => {
        console.warn(
          `command [get_connection_state] returned an error: ${e.source} - ${e.message}`,
        );
      });

    getSessionStartTime()
      .then((startTime) =>
        dispatch({ type: 'set-connection-start-time', startTime }),
      )
      .catch((e: CmdError) => {
        console.warn(
          `command [get_connection_start_time] returned an error: ${e.source} - ${e.message}`,
        );
      });

    getCountries()
      .then((countries) => {
        dispatch({ type: 'set-countries', countries });
      })
      .catch((e: CmdError) => {
        console.warn(
          `command [get_node_countries] returned an error: ${e.source} - ${e.message}`,
        );
      });

    getDefaultNodeLocation()
      .then((country) => {
        dispatch({ type: 'set-default-node-location', country });
      })
      .catch((e: CmdError) => {
        console.warn(
          `command [get_default_node_location] returned an error: ${e.source} - ${e.message}`,
        );
      });
  }, []);

  // get saved on disk app data and restore state from it
  useEffect(() => {
    const getAppData = async () => {
      return await invoke<AppDataFromBackend>('get_app_data');
    };

    getAppData()
      .then((data) => {
        console.log('app data read from disk:');
        console.log(data);

        if (data.ui_root_font_size) {
          document.documentElement.style.fontSize = `${data.ui_root_font_size}px`;
        }

        const partialState: Partial<typeof initialState> = {
          entrySelector: data.entry_location_selector || false,
          uiTheme: data.ui_theme || 'Light',
          vpnMode: data.vpn_mode || 'TwoHop',
          autoConnect: data.autoconnect || false,
          monitoring: data.monitoring || false,
          rootFontSize: data.ui_root_font_size || DefaultRootFontSize,
        };
        if (data.entry_node_location) {
          partialState.entryNodeLocation = data.entry_node_location;
        }
        if (data.exit_node_location) {
          partialState.exitNodeLocation = data.exit_node_location;
        }
        dispatch({
          type: 'set-partial-state',
          partialState,
        });
      })
      .catch((e: CmdError) => {
        console.warn(
          `command [get_app_data] returned an error: ${e.source} - ${e.message}`,
        );
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
