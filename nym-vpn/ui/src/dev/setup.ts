import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { AppDataFromBackend, ConnectionState } from '../types';
import { emit } from '@tauri-apps/api/event';
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
    if (cmd === 'get_app_data') {
      return new Promise<AppDataFromBackend>((resolve) =>
        resolve({
          monitoring: false,
          autoconnect: false,
          killswitch: false,
          ui_mode: 'Dark',
          privacy_mode: 'High',
          entry_node: null,
          exit_node: null,
        }),
      );
    }
  });
}
