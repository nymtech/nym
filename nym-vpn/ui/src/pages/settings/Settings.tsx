import { useEffect, useState } from 'react';
import clsx from 'clsx';
import { invoke } from '@tauri-apps/api';
import { Switch } from '@headlessui/react';
import { useTranslation } from 'react-i18next';
import { useMainDispatch, useMainState } from '../../contexts';
import { StateDispatch } from '../../types';
import { QuickConnectCountry } from '../../constants';

function Settings() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const { t } = useTranslation('settings');

  const [darkModeEnabled, setDarkModeEnabled] = useState(
    state.uiTheme === 'Dark',
  );
  const [entrySelector, setEntrySelector] = useState(state.entrySelector);

  useEffect(() => {
    setDarkModeEnabled(state.uiTheme === 'Dark');
    setEntrySelector(state.entrySelector);
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

  const handleDefaultEntryNodeSelection = async () => {
    try {
      await invoke<void>('set_node_location', {
        nodeType: 'Entry',
        country: QuickConnectCountry,
      });
      dispatch({
        type: 'set-node-location',
        payload: { hop: 'entry', country: QuickConnectCountry },
      });
    } catch (e) {
      console.log(e);
    }
  };

  const handleEntrySelectorChange = async () => {
    const isSelected = !state.entrySelector;
    dispatch({ type: 'set-entry-selector', entrySelector: isSelected });
    invoke<void>('set_entry_selector', { entrySelector: isSelected }).catch(
      (e) => {
        console.log(e);
      },
    );
    if (!isSelected) {
      handleDefaultEntryNodeSelection();
    }
  };

  return (
    <div className="h-full flex flex-col p-4 gap-4">
      <div className="flex flex-row justify-between items-center">
        <p className="text-lg text-baltic-sea dark:text-mercury-pinkish">
          Dark Mode
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
              'inline-block h-4 w-4 transform rounded-full bg-ciment-feet dark:bg-white transition',
            ])}
          />
        </Switch>
      </div>
      <div className="flex flex-row justify-between items-center">
        <p className="text-lg text-baltic-sea dark:text-mercury-pinkish">
          {t('entry-selector')}
        </p>
        <Switch
          checked={entrySelector}
          onChange={handleEntrySelectorChange}
          className={clsx([
            entrySelector
              ? 'bg-melon'
              : 'bg-mercury-pinkish dark:bg-gun-powder',
            'relative inline-flex h-6 w-11 items-center rounded-full',
          ])}
        >
          <span className="sr-only">{t('entry-selector')}</span>
          <span
            className={clsx([
              entrySelector ? 'translate-x-6' : 'translate-x-1',
              'inline-block h-4 w-4 transform rounded-full bg-ciment-feet dark:bg-white transition',
            ])}
          />
        </Switch>
      </div>
    </div>
  );
}

export default Settings;
