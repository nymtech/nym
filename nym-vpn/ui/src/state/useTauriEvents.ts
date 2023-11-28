import { useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  ConnectionEventPayload,
  ProgressEventPayload,
  StateDispatch,
} from '../types';
import { ConnectionEvent, ProgressEvent } from '../constants';
import dayjs from 'dayjs';

function handleError(dispatch: StateDispatch, error?: string | null) {
  if (!error) {
    dispatch({ type: 'reset-error' });
    return;
  }
  dispatch({ type: 'set-error', error });
}

export function useTauriEvents(dispatch: StateDispatch) {
  const registerStateListener = useCallback(() => {
    return listen<ConnectionEventPayload>(ConnectionEvent, (event) => {
      console.log(
        `received event ${event.event}, state: ${event.payload.state}`,
      );
      switch (event.payload.state) {
        case 'Connected':
          dispatch({
            type: 'set-connected',
            startTime: event.payload.start_time || dayjs().unix(),
          });
          handleError(dispatch, event.payload.error);
          break;
        case 'Disconnected':
          dispatch({ type: 'set-disconnected' });
          handleError(dispatch, event.payload.error);
          break;
        case 'Connecting':
          dispatch({ type: 'change-connection-state', state: 'Connecting' });
          handleError(dispatch, event.payload.error);
          break;
        case 'Disconnecting':
          dispatch({ type: 'change-connection-state', state: 'Disconnecting' });
          handleError(dispatch, event.payload.error);
          break;
        case 'Unknown':
          dispatch({ type: 'change-connection-state', state: 'Unknown' });
          handleError(dispatch, event.payload.error);
          break;
      }
    });
  }, [dispatch]);

  const registerProgressListener = useCallback(() => {
    return listen<ProgressEventPayload>(ProgressEvent, (event) => {
      console.log(
        `received event ${event.event}, message: ${event.payload.message}`,
      );
      dispatch({
        type: 'new-progress-message',
        message: event.payload.message,
      });
    });
  }, [dispatch]);

  // register/unregister event listener
  useEffect(() => {
    const unlistenState = registerStateListener();
    const unlistenProgress = registerProgressListener();

    return () => {
      unlistenState.then((f) => f());
      unlistenProgress.then((f) => f());
    };
  }, [registerStateListener, registerProgressListener]);
}
