import { useCallback, useEffect, useReducer } from 'react';
import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { initialState, reducer } from './main';
import { MainDispatchContext, MainStateContext } from '../contexts';
import { ConnectionState, EventPayload } from '../types';

const ConnectionEvent = 'connection-state';

type Props = {
  children?: React.ReactNode;
};

export function MainStateProvider({ children }: Props) {
  const [state, dispatch] = useReducer(reducer, initialState);

  const registerListener = useCallback(async () => {
    return await listen<EventPayload>(ConnectionEvent, (event) => {
      console.log(
        `received event ${event.event}, state: ${event.payload.state}`,
      );
      switch (event.payload.state) {
        case 'Connected':
          dispatch({ type: 'change-connection-state', state: 'Connected' });
          break;
        case 'Disconnected':
          dispatch({ type: 'change-connection-state', state: 'Disconnected' });
          break;
        case 'Connecting':
          dispatch({ type: 'change-connection-state', state: 'Connecting' });
          break;
        case 'Disconnecting':
          dispatch({ type: 'change-connection-state', state: 'Disconnecting' });
          break;
        case 'Error':
          break;
      }
    });
  }, []);

  // register/unregister event listener
  useEffect(() => {
    let unlisten = () => {};
    registerListener().then((fn) => (unlisten = fn));

    return () => {
      unlisten();
    };
  }, [registerListener]);

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
