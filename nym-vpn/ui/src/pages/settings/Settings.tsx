import { useEffect, useState } from 'react';
import clsx from 'clsx';
import { invoke } from '@tauri-apps/api';
import { Switch } from '@headlessui/react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { routes } from '../../constants';
import { useMainDispatch, useMainState } from '../../contexts';
import { StateDispatch } from '../../types';
import SettingsGroup from './SettingsGroup';

function Settings() {
  const state = useMainState();
  const navigate = useNavigate();
  const dispatch = useMainDispatch() as StateDispatch;
  const { t } = useTranslation('settings');

  const [entrySelector, setEntrySelector] = useState(state.entrySelector);
  const [autoConnect, setAutoConnect] = useState(state.autoConnect);
  const [monitoring, setMonitoring] = useState(state.monitoring);

  useEffect(() => {
    setEntrySelector(state.entrySelector);
    setAutoConnect(state.autoConnect);
    setMonitoring(state.monitoring);
  }, [state]);

  const handleEntrySelectorChange = async () => {
    const isSelected = !state.entrySelector;
    dispatch({ type: 'set-entry-selector', entrySelector: isSelected });
    invoke<void>('set_entry_location_selector', {
      entrySelector: isSelected,
    }).catch((e) => {
      console.log(e);
    });
  };

  const handleAutoConnectChanged = async () => {
    const isSelected = !state.autoConnect;
    dispatch({ type: 'set-auto-connect', autoConnect: isSelected });
    invoke<void>('set_auto_connect', { autoConnect: isSelected }).catch((e) => {
      console.log(e);
    });
  };

  const handleMonitoringChanged = async () => {
    const isSelected = !state.monitoring;
    dispatch({ type: 'set-monitoring', monitoring: isSelected });
    invoke<void>('set_monitoring', { monitoring: isSelected }).catch((e) => {
      console.log(e);
    });
  };

  return (
    <div className="h-full flex flex-col p-4 mt-2 gap-8">
      <SettingsGroup
        settings={[
          {
            title: t('auto-connect.title'),
            desc: t('auto-connect.desc'),
            leadingIcon: 'hdr_auto',
            disabled: true,
            trailing: (
              <Switch
                checked={autoConnect}
                onChange={handleAutoConnectChanged}
                className={clsx([
                  autoConnect
                    ? 'bg-melon'
                    : 'bg-mercury-pinkish dark:bg-gun-powder',
                  'relative inline-flex h-6 w-11 items-center rounded-full',
                ])}
                disabled
              >
                <span
                  className={clsx([
                    autoConnect ? 'translate-x-6' : 'translate-x-1',
                    'inline-block h-4 w-4 transform rounded-full bg-cement-feet dark:bg-white transition',
                  ])}
                />
              </Switch>
            ),
          },
          {
            title: t('entry-selector.title'),
            desc: t('entry-selector.desc'),
            leadingIcon: 'looks_two',
            disabled: true,
            trailing: (
              <Switch
                checked={entrySelector}
                onChange={handleEntrySelectorChange}
                className={clsx([
                  entrySelector
                    ? 'bg-melon'
                    : 'bg-mercury-pinkish dark:bg-gun-powder',
                  'relative inline-flex h-6 w-11 items-center rounded-full',
                ])}
                disabled
              >
                <span
                  className={clsx([
                    entrySelector ? 'translate-x-6' : 'translate-x-1',
                    'inline-block h-4 w-4 transform rounded-full bg-cement-feet dark:bg-white transition',
                  ])}
                />
              </Switch>
            ),
          },
        ]}
      />
      <SettingsGroup
        settings={[
          {
            title: t('display-theme'),
            leadingIcon: 'contrast',
            onClick: () => {
              navigate(routes.display);
            },
            trailing: (
              <div className="font-icon text-2xl cursor-pointer">
                arrow_right
              </div>
            ),
          },
        ]}
      />
      <SettingsGroup
        settings={[
          {
            title: t('logs'),
            leadingIcon: 'sort',
            onClick: () => {
              navigate(routes.logs);
            },
            trailing: (
              <div className="font-icon text-2xl cursor-pointer">
                arrow_right
              </div>
            ),
            disabled: true,
          },
        ]}
      />
      <SettingsGroup
        settings={[
          {
            title: t('feedback'),
            leadingIcon: 'question_answer',
            onClick: () => {
              navigate(routes.feedback);
            },
            trailing: (
              <div className="font-icon text-2xl cursor-pointer">
                arrow_right
              </div>
            ),
            disabled: true,
          },
          {
            title: t('error-reporting.title'),
            desc: t('error-reporting.desc'),
            leadingIcon: 'error',
            disabled: true,
            trailing: (
              <Switch
                checked={monitoring}
                onChange={handleMonitoringChanged}
                className={clsx([
                  monitoring
                    ? 'bg-melon'
                    : 'bg-mercury-pinkish dark:bg-gun-powder',
                  'relative inline-flex h-6 w-11 items-center rounded-full',
                ])}
                disabled
              >
                <span
                  className={clsx([
                    monitoring ? 'translate-x-6' : 'translate-x-1',
                    'inline-block h-4 w-4 transform rounded-full bg-cement-feet dark:bg-white transition',
                  ])}
                />
              </Switch>
            ),
          },
          {
            title: t('faq'),
            leadingIcon: 'help',
            disabled: true,
            trailing: (
              <div className="font-icon text-2xl cursor-pointer">launch</div>
            ),
          },
        ]}
      />
      <SettingsGroup
        settings={[
          {
            title: t('legal'),
            onClick: () => {
              navigate(routes.legal);
            },
            disabled: true,
            trailing: (
              <div className="font-icon text-2xl cursor-pointer">
                arrow_right
              </div>
            ),
          },
        ]}
      />
      <SettingsGroup
        settings={[
          {
            title: t('quit'),
            onClick: () => {
              //TODO shutdown gracefully
            },
            disabled: true,
          },
        ]}
      />
      <div className="text-comet text-sm tracking-tight leading-tight">
        Version {state.version}
      </div>
    </div>
  );
}

export default Settings;
