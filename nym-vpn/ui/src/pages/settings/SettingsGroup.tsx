import clsx from 'clsx';
import { ReactNode } from 'react';
import { RadioGroup } from '@headlessui/react';

type Setting = {
  title: string;
  leadingIcon: string | null | ReactNode;
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
              onClick={setting.onClick}
              className={clsx([
                'bg-white dark:bg-baltic-sea-jaguar relative flex px-5 py-2 shadow-md focus:outline-none',
                index === 0 && 'rounded-t-lg',
                index === settings.length - 1 &&
                  settings.length === 2 &&
                  'border-t border-mercury-mist',
                index !== 0 &&
                  index !== settings.length - 1 &&
                  'border-y border-mercury-mist',
                index === settings.length - 1 && 'rounded-b-lg',
                setting.desc === '' && 'py-4',
              ])}
            >
              <div className="flex flex-1 items-center justify-between gap-4">
                {setting.leadingIcon ? (
                  <span className="font-icon text-2xl">
                    {setting.leadingIcon}
                  </span>
                ) : (
                  <></>
                )}
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
                <div>{setting.trailing}</div>
              </div>
            </RadioGroup.Option>
          ))}
        </div>
      </RadioGroup>
    </div>
  );
}

export default SettingsGroup;
