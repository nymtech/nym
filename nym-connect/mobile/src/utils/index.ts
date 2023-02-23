import { useEffect, useRef } from 'react';
import { EventName, listen, UnlistenFn, EventCallback } from '@tauri-apps/api/event';

export const useTauriEvents = <T>(event: EventName, handler: EventCallback<T>) => {
  const unlisten = useRef<UnlistenFn>();

  // list for events to clear local storage
  useEffect(() => {
    listen(event, handler).then((fn) => {
      unlisten.current = fn;
    });

    return () => {
      if (unlisten.current) {
        unlisten.current();
      }
    };
  }, []);
};
