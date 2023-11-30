import { useState } from 'react';
import { RadioGroup } from '@headlessui/react';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { useMainDispatch, useMainState } from '../../contexts';
import { StateDispatch, VpnMode } from '../../types';

type VpnModeOption = { name: VpnMode; title: string; desc: string };

function NetworkModeSelect() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const { t } = useTranslation('home');

  const handleNetworkModeChange = (value: VpnMode) => {
    if (state.state === 'Disconnected') {
      dispatch({ type: 'set-vpn-mode', mode: value });
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
  const [selected, setSelected] = useState(vpnModes[0].name);

  const handleSelect = (value: VpnMode) => {
    setSelected(value);
    handleNetworkModeChange(value);
  };

  return (
    <div className="">
      <RadioGroup value={selected} onChange={handleSelect}>
        <RadioGroup.Label
          as="div"
          className="font-semibold text-lg text-baltic-sea dark:text-white mb-4"
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
                  'bg-white dark:bg-baltic-sea-jaguar relative flex cursor-pointer rounded-lg px-5 py-3 shadow-md focus:outline-none',
                  checked &&
                    'ring-0 ring-melon ring-offset-2 ring-offset-melon',
                ])
              }
            >
              {({ checked }) => {
                return (
                  <div className="flex flex-1 items-center justify-between gap-4">
                    {checked ? (
                      <span className="font-icon text-2xl text-melon">
                        radio_button_checked
                      </span>
                    ) : (
                      <span className="font-icon text-2xl text-mercury-pinkish">
                        radio_button_unchecked
                      </span>
                    )}
                    <div className="flex flex-1 items-center">
                      <div className="text-sm">
                        <RadioGroup.Label
                          as="p"
                          className="font-semibold text-lg text-baltic-sea dark:text-mercury-pinkish"
                        >
                          {mode.title}
                        </RadioGroup.Label>
                        <RadioGroup.Description
                          as="span"
                          className="text-base text-ciment-feet dark:text-mercury-mist"
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
