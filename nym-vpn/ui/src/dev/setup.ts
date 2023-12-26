import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { AppDataFromBackend, ConnectionState, Country } from '../types';
import { ConnectionEvent } from '../constants';

export function mockTauriIPC() {
  mockWindows('main');

  mockIPC(async (cmd, args) => {
    console.log(`IPC call mocked "${cmd}"`);
    console.log(args);
    if (cmd === 'connect') {
      await emit(ConnectionEvent, { state: 'Connecting' });
      return new Promise<ConnectionState>((resolve) =>
        setTimeout(async () => {
          await emit(ConnectionEvent, { state: 'Connected' });
          resolve('Connected');
        }, 2000),
      );
    }
    if (cmd === 'disconnect') {
      await emit(ConnectionEvent, { state: 'Disconnecting' });
      return new Promise<ConnectionState>((resolve) =>
        setTimeout(async () => {
          await emit(ConnectionEvent, { state: 'Disconnected' });
          resolve('Disconnected');
        }, 2000),
      );
    }
    if (cmd === 'get_connection_state') {
      return new Promise<ConnectionState>((resolve) =>
        setTimeout(() => resolve('Disconnected'), 2000),
      );
    }

    if (cmd === 'get_node_countries') {
      return new Promise<Country[]>((resolve) =>
        resolve([
          {
            name: 'United States',
            code: 'US',
          },
          {
            name: 'France',
            code: 'FR',
          },
          {
            name: 'Switzerland',
            code: 'CH',
          },
          {
            name: 'Germany',
            code: 'DE',
          },
        ]),
      );
    }

    if (cmd === 'get_default_node_location') {
      return new Promise<Country>((resolve) =>
        resolve({
          name: 'France',
          code: 'FR',
        }),
      );
    }

    if (cmd === 'set_root_font_size') {
      return new Promise<void>((resolve) => resolve());
    }

    if (cmd === 'get_app_data') {
      return new Promise<AppDataFromBackend>((resolve) =>
        resolve({
          monitoring: false,
          autoconnect: false,
          killswitch: false,
          entry_location_selector: false,
          ui_theme: 'Dark',
          ui_root_font_size: 12,
          vpn_mode: 'TwoHop',
          entry_node: {
            country: {
              name: 'France',
              code: 'FR',
            },
            id: 'nodeOne',
          },
          exit_node: {
            country: {
              name: 'France',
              code: 'FR',
            },
            id: 'nodeTwo',
          },
          entry_node_location: null,
          exit_node_location: null,
        }),
      );
    }
  });
}
