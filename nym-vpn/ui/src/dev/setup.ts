import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { greet } from './tauri-cmd-mocks';

mockWindows('main');

mockIPC(async (cmd, args) => {
  console.log(`IPC call mocked "${cmd}"`);
  if (cmd === 'greet') {
    return greet(args.name as string);
  }
});
