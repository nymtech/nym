import { useEffect, useRef } from 'react';
import { EventName, listen, UnlistenFn, EventCallback } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api';

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

export const toggle_log_viewer = async () => {
  await invoke('help_log_toggle_window');
};
