import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { AppDataFromBackend, ConnectionState, Country } from '../types';
import { ConnectionEvent, quickConnectCountry } from '../constants';

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
      return new Promise<Array<Country>>((resolve) =>
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

    if (cmd === 'get_app_data') {
      return new Promise<AppDataFromBackend>((resolve) =>
        resolve({
          monitoring: false,
          autoconnect: false,
          killswitch: false,
          ui_theme: 'Dark',
          vpn_mode: 'TwoHop',
          entry_node: {
            country: quickConnectCountry.name,
            id: quickConnectCountry.code,
          },
          exit_node: {
            country: quickConnectCountry.name,
            id: quickConnectCountry.code,
          },
        }),
      );
    }
  });
}
