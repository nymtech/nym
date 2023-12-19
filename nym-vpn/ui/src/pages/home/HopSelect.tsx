import { useTranslation } from 'react-i18next';
import { Country, NodeHop } from '../../types';
import { QuickConnectPrefix } from '../../constants';
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
    <>
      <div
        className="relative w-full flex flex-row justify-center"
        onKeyDown={onClick}
        role="presentation"
        onClick={onClick}
      >
        <input
          readOnly={true}
          type="text"
          id="floating_outlined"
          value={country.name}
          className="disabled:cursor-default dark:bg-baltic-sea cursor-pointer pl-11 dark:placeholder-white border border-gun-powder block px-2.5 pb-4 pt-4 w-full text-sm text-gray-900 bg-transparent rounded-lg border-1 border-gray-300 appearance-none dark:text-white dark:border-gray-600 dark:focus:border-blue-500 focus:outline-none focus:ring-0 focus:border-blue-600 peer"
          disabled={state !== 'Disconnected'}
        />
        <div className="top-1/2 transform -translate-y-1/2 left-2 absolute pointer-events-none">
          {country.name.includes(QuickConnectPrefix) ? (
            <span className="font-icon px-2">bolt</span>
          ) : (
            <img
              src={`./flags/${country.code.toLowerCase()}.svg`}
              className="h-8 scale-75 pointer-events-none fill-current"
              alt={country.code}
            />
          )}
        </div>

        <span className="font-icon scale-125 pointer-events-none absolute fill-current top-1/4 transform -translate-x-1/2 right-2">
          arrow_right
        </span>
        <label
          htmlFor="floating_outlined"
          className="dark:text-white bg-blanc-nacre dark:bg-baltic-sea absolute text-sm text-gray-500 dark:text-gray-400 ml-4 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] dark:bg-gray-900 px-2 peer-placeholder-shown:px-2 peer-placeholder-shown:text-blue-600 peer-placeholder-shown:dark:text-blue-500 peer-placeholder-shown:top-2 peer-placeholder-shown:scale-75 peer-placeholder-shown:-translate-y-4 rtl:peer-placeholder-shown:translate-x-1/4 rtl:peer-placeholder-shown:left-auto start-1"
        >
          {nodeHop === 'entry' ? t('first-hop') : t('last-hop')}
        </label>
      </div>
    </>
  );
}
