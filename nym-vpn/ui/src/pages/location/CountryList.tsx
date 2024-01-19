import { useTranslation } from 'react-i18next';
import clsx from 'clsx';
import { Country } from '../../types';

interface CountryListProps {
  countries: Country[];
  onClick: (name: string, code: string) => void;
  isSelected: (code: string) => boolean;
}

export default function CountryList({
  countries,
  onClick,
  isSelected,
}: CountryListProps) {
  const { t } = useTranslation('nodeLocation');

  return (
    <ul className="flex flex-col w-full items-stretch gap-1">
      {countries && countries.length > 0 ? (
        countries.map((country) => (
          <li key={country.code} className="list-none w-full">
            <div
              role="presentation"
              onKeyDown={() => onClick(country.name, country.code)}
              className={clsx([
                'flex flex-row justify-between',
                'hover:bg-gun-powder hover:bg-opacity-10',
                'dark:hover:bg-laughing-jack dark:hover:bg-opacity-10',
                'rounded-lg cursor-pointer px-3 py-1',
                isSelected(country.code) &&
                  'bg-gun-powder dark:bg-laughing-jack bg-opacity-15 dark:bg-opacity-15',
              ])}
              onClick={() => onClick(country.name, country.code)}
            >
              <div className="flex flex-row items-center m-1 gap-3 p-1 cursor-pointer">
                <img
                  src={`./flags/${country.code.toLowerCase()}.svg`}
                  className="h-6"
                  alt={country.code}
                />
                <div className="flex items-center dark:text-mercury-pinkish text-base cursor-pointer">
                  {country.name}
                </div>
              </div>
              <div
                className={clsx([
                  'pr-4 flex items-center font-medium text-xs cursor-pointer',
                  'text-cement-feet dark:text-mercury-mist',
                ])}
              >
                {isSelected(country.code) && t('selected')}
              </div>
            </div>
          </li>
        ))
      ) : (
        <p className="flex justify-center">{t('none-found')}</p>
      )}
    </ul>
  );
}
