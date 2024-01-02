import clsx from 'clsx';
import { ReactNode } from 'react';
import { RadioGroup } from '@headlessui/react';

type Setting = {
  title: string;
  leadingIcon: string;
  desc: string;
  onClick: () => void;
  trailing: ReactNode;
};

interface Props {
  settings: Setting[];
}

function SettingsGroup({ settings }: Props) {
  return (
    <div>
      <RadioGroup>
        <div>
          {settings.map((setting, index) => (
            <RadioGroup.Option
              key={setting.title}
              value={setting.title}
              className={clsx([
                'bg-white dark:bg-baltic-sea-jaguar relative flex px-5 py-3 shadow-md focus:outline-none',
                index === 0 && 'rounded-t-lg',
                index === settings.length - 1 &&
                  settings.length === 2 &&
                  'border-t border-mercury-mist',
                index !== 0 &&
                  index !== settings.length - 1 &&
                  'border-y border-mercury-mist',
                index === settings.length - 1 && 'rounded-b-lg',
              ])}
            >
              <div className="flex flex-1 items-center justify-between gap-4">
                <span className="font-icon text-2xl">
                  {setting.leadingIcon}
                </span>
                <div className="flex flex-1 items-center">
                  <div className="text-sm">
                    <RadioGroup.Label
                      as="p"
                      className="font-semibold text-lg text-baltic-sea dark:text-mercury-pinkish"
                    >
                      {setting.title}
                    </RadioGroup.Label>
                    <RadioGroup.Description
                      as="span"
                      className="text-base text-cement-feet dark:text-mercury-mist"
                    >
                      <span>{setting.desc}</span>
                    </RadioGroup.Description>
                  </div>
                </div>
                <span className="text-2xl" onClick={setting.onClick}>
                  {setting.trailing}
                </span>
              </div>
            </RadioGroup.Option>
          ))}
        </div>
      </RadioGroup>
    </div>
  );
}

export default SettingsGroup;
