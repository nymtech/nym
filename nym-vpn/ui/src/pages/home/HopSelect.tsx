import { useTranslation } from 'react-i18next';
import clsx from 'clsx';
import { Country, NodeHop } from '../../types';
import { useMainState } from '../../contexts';

interface HopSelectProps {
  country: Country;
  onClick: () => void;
  nodeHop: NodeHop;
}

export default function HopSelect({
  nodeHop,
  country,
  onClick,
}: HopSelectProps) {
  const { state } = useMainState();
  const { t } = useTranslation('home');

  return (
    <div
      className={clsx([
        state === 'Disconnected' ? 'cursor-pointer' : 'cursor-not-allowed',
        'w-full flex flex-row justify-between items-center py-3 px-4',
        'text-baltic-sea dark:text-mercury-pinkish',
        'border-cement-feet dark:border-gun-powder border-2 rounded-lg',
        'relative',
      ])}
      onKeyDown={onClick}
      role="presentation"
      onClick={onClick}
    >
      <div
        className={clsx([
          'absolute left-3 -top-3 px-1',
          'bg-blanc-nacre dark:bg-baltic-sea text-sm',
        ])}
      >
        {nodeHop === 'entry' ? t('first-hop') : t('last-hop')}
      </div>
      <div className="flex flex-row items-center gap-3">
        <img
          src={`./flags/${country.code.toLowerCase()}.svg`}
          className="h-8 scale-90 pointer-events-none fill-current"
          alt={country.code}
        />
        <div className="text-base">{country.name}</div>
      </div>
      <span className="font-icon text-2xl pointer-events-none">
        arrow_right
      </span>
    </div>
  );
}
