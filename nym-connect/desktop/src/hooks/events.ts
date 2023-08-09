import { useEffect, useRef } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import * as Sentry from '@sentry/react';
import { ConnectionStatusKind, GatewayPerformance } from 'src/types';
import { Error } from 'src/types/error';
import { TauriEvent } from 'src/types/event';

const TAURI_EVENT_STATUS_CHANGED = 'app:connection-status-changed';

export const useEvents = ({
  onError,
  onStatusChange,
  onGatewayPerformanceChange,
}: {
  onError: (error: Error) => void;
  onStatusChange: (status: ConnectionStatusKind) => void;
  onGatewayPerformanceChange: (status: GatewayPerformance) => void;
}) => {
  const timerId = useRef<NodeJS.Timeout>();

  useEffect(() => {
    const unlisten: UnlistenFn[] = [];

    // TODO: fix typings
    listen(TAURI_EVENT_STATUS_CHANGED, (event) => {
      const { status } = event.payload as any;
      console.log(TAURI_EVENT_STATUS_CHANGED, { status, event });
      onStatusChange(status);
    })
      .then((result) => {
        unlisten.push(result);
      })
      .catch((e) => {
        console.log(e);
        Sentry.captureException(e);
      });

    listen('socks5-event', (e: TauriEvent) => {
      console.log(e);
      onError(e.payload);
    }).then((result) => {
      unlisten.push(result);
    });

    listen('socks5-status-event', (e: TauriEvent) => {
      if (e.payload.message.includes('slow')) {
        onGatewayPerformanceChange('Poor');

        if (timerId?.current) {
          clearTimeout(timerId.current);
        }

        timerId.current = setTimeout(() => {
          onGatewayPerformanceChange('Good');
        }, 10000);
      }
    }).then((result) => {
      unlisten.push(result);
    });

    listen('socks5-connection-fail-event', (e: TauriEvent) => {
      onError({ title: 'Connection failed', message: `${e.payload.message} - Please disconnect and reconnect.` });
      onGatewayPerformanceChange('Poor');
    }).then((result) => {
      unlisten.push(result);
    });

    return () => {
      unlisten.forEach((unsubscribe) => unsubscribe());
    };
  }, []);
};
