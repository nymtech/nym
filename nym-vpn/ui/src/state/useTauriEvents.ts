import { useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { EventPayload, StateDispatch } from '../types';
import { ConnectionEvent } from '../constants';

export function useTauriEvents(dispatch: StateDispatch) {
  const registerListener = useCallback(() => {
    return listen<EventPayload>(ConnectionEvent, (event) => {
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
  }, [dispatch]);

  // register/unregister event listener
  useEffect(() => {
    const unlisten = registerListener();

    return () => {
      unlisten.then((f) => f());
    };
  }, [registerListener]);
}
