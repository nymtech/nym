import { useEffect, useState } from 'react';
import { RadioGroup } from '@headlessui/react';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { useMainDispatch, useMainState } from '../../contexts';
import { StateDispatch, VpnMode } from '../../types';

type VpnModeOption = { name: VpnMode; title: string; desc: string };

function NetworkModeSelect() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;
  const [selected, setSelected] = useState(state.vpnMode);
  const [loading, setLoading] = useState(false);

  const { t } = useTranslation('home');

  useEffect(() => {
    if (state.vpnMode !== selected) {
      setSelected(state.vpnMode);
    }
  }, [state.vpnMode, selected]);

  const handleNetworkModeChange = async (value: VpnMode) => {
    if (state.state === 'Disconnected' && value !== state.vpnMode) {
      setLoading(true);
      try {
        await invoke<void>('set_vpn_mode', { mode: value });
        dispatch({ type: 'set-vpn-mode', mode: value });
      } catch (e) {
        console.log(e);
      } finally {
        setLoading(false);
      }
    }
  };

  const vpnModes: VpnModeOption[] = [
    {
      name: 'Mixnet',
      title: t('mixnet-mode.title'),
      desc: t('mixnet-mode.desc'),
    },
    {
      name: 'TwoHop',
      title: t('twohop-mode.title'),
      desc: t('twohop-mode.desc'),
    },
  ];

  const handleSelect = (value: VpnMode) => {
    setSelected(value);
    handleNetworkModeChange(value);
  };

  return (
    <div>
      <RadioGroup value={selected} onChange={handleSelect}>
        <RadioGroup.Label
          as="div"
          className="font-semibold text-base text-baltic-sea dark:text-white mb-6"
        >
          {t('select-network-label')}
        </RadioGroup.Label>
        <div className="space-y-4">
          {vpnModes.map((mode) => (
            <RadioGroup.Option
              key={mode.name}
              value={mode.name}
              className={({ checked }) =>
                clsx([
                  'bg-white dark:bg-baltic-sea-jaguar relative flex rounded-lg px-5 py-2 focus:outline-none',
                  (state.state !== 'Disconnected' || loading) &&
                    'cursor-not-allowed',
                  checked &&
                    'ring-0 ring-melon ring-offset-2 ring-offset-melon',
                  state.state === 'Disconnected' && 'cursor-pointer',
                ])
              }
              disabled={state.state !== 'Disconnected' || loading}
            >
              {({ checked }) => {
                return (
                  <div className="flex flex-1 items-center justify-between gap-4">
                    {checked ? (
                      <span className="font-icon text-2xl text-melon">
                        radio_button_checked
                      </span>
                    ) : (
                      <span className="font-icon text-2xl text-cement-feet dark:laughing-jack">
                        radio_button_unchecked
                      </span>
                    )}
                    <div className="flex flex-1 items-center">
                      <div className="text-sm">
                        <RadioGroup.Label
                          as="p"
                          className={clsx([
                            'text-base text-baltic-sea dark:text-mercury-pinkish',
                            checked && 'font-semibold',
                          ])}
                        >
                          {mode.title}
                        </RadioGroup.Label>
                        <RadioGroup.Description
                          as="span"
                          className="text-sm text-cement-feet dark:text-mercury-mist"
                        >
                          <span>{mode.desc}</span>
                        </RadioGroup.Description>
                      </div>
                    </div>
                  </div>
                );
              }}
            </RadioGroup.Option>
          ))}
        </div>
      </RadioGroup>
    </div>
  );
}

export default NetworkModeSelect;
