import { useEffect, useState } from 'react';
import clsx from 'clsx';
import { invoke } from '@tauri-apps/api';
import { Switch } from '@headlessui/react';
import { useMainDispatch, useMainState } from '../../contexts';
import { StateDispatch } from '../../types';

function Settings() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const [enabled, setEnabled] = useState(state.uiTheme === 'Dark');

  useEffect(() => {
    setEnabled(state.uiTheme === 'Dark');
  }, [state.uiTheme]);

  const handleThemeChange = async (darkMode: boolean) => {
    if (darkMode && state.uiTheme === 'Light') {
      dispatch({ type: 'set-ui-theme', theme: 'Dark' });
    } else if (!darkMode && state.uiTheme === 'Dark') {
      dispatch({ type: 'set-ui-theme', theme: 'Light' });
    }
    invoke<void>('set_ui_theme', { theme: darkMode ? 'Dark' : 'Light' }).catch(
      (e) => {
        console.log(e);
      },
    );
  };

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex flex-row justify-between items-center">
        <p className="text-lg text-baltic-sea dark:text-mercury-pinkish">
          Dark Mode
        </p>
        <Switch
          checked={enabled}
          onChange={handleThemeChange}
          className={clsx([
            enabled ? 'bg-melon' : 'bg-mercury-pinkish dark:bg-gun-powder',
            'relative inline-flex h-6 w-11 items-center rounded-full',
          ])}
        >
          <span className="sr-only">Dark mode</span>
          <span
            className={clsx([
              enabled ? 'translate-x-6' : 'translate-x-1',
              'inline-block h-4 w-4 transform rounded-full bg-white transition',
            ])}
          />
        </Switch>
      </div>
    </div>
  );
}

export default Settings;
