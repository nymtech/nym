import { useEffect, useState } from 'react';
import clsx from 'clsx';
import { invoke } from '@tauri-apps/api';
import { Switch } from '@headlessui/react';
import { useTranslation } from 'react-i18next';
import { useMainDispatch, useMainState } from '../../../contexts';
import { StateDispatch } from '../../../types';
import UiScaler from './UiScaler';

function Display() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;
  const { t } = useTranslation();

  const [darkModeEnabled, setDarkModeEnabled] = useState(
    state.uiTheme === 'Dark',
  );

  useEffect(() => {
    setDarkModeEnabled(state.uiTheme === 'Dark');
  }, [state]);

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
    <div className="h-full flex flex-col py-6 gap-6">
      <div
        className={clsx([
          'flex flex-row justify-between items-center',
          'bg-white dark:bg-baltic-sea-jaguar',
          'px-6 py-4 rounded-lg',
        ])}
      >
        <p className="text-base text-baltic-sea dark:text-mercury-pinkish select-none">
          {t('ui-mode.dark')}
        </p>
        <Switch
          checked={darkModeEnabled}
          onChange={handleThemeChange}
          className={clsx([
            darkModeEnabled
              ? 'bg-melon'
              : 'bg-mercury-pinkish dark:bg-gun-powder',
            'relative inline-flex h-6 w-11 items-center rounded-full',
          ])}
        >
          <span className="sr-only">Dark mode</span>
          <span
            className={clsx([
              darkModeEnabled ? 'translate-x-6' : 'translate-x-1',
              'inline-block h-4 w-4 transform rounded-full bg-cement-feet dark:bg-white transition',
            ])}
          />
        </Switch>
      </div>
      <UiScaler />
    </div>
  );
}

export default Display;
