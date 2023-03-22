import { useEffect, useRef } from 'react';
import { EventName, listen, UnlistenFn, EventCallback } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api';
import { forage } from '@tauri-apps/tauri-forage';
import { StorageKeyValue } from 'src/types/storage';

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

export const toggleLogViewer = async () => {
  await invoke('help_log_toggle_window');
};

export async function setItemInStorage<T>({ key, value }: StorageKeyValue<T>) {
  try {
    await forage.setItem({
      key,
      value,
    } as any)();
  } catch (e) {
    console.warn(e);
  }
  return undefined;
}

export const getItemFromStorage = async ({ key }: Pick<StorageKeyValue<undefined>, 'key'>) => {
  try {
    const gatewayFromStorage = await forage.getItem({ key })();
    return gatewayFromStorage;
  } catch (e) {
    console.warn(e);
  }
  return undefined;
};
