import { useTranslation } from 'react-i18next';
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
    <ul className="flex flex-col w-full items-stretch gap-4">
      {countries && countries.length > 0 ? (
        countries.map((country) => (
          <li key={country.code} className="list-none w-full">
            <div
              role="presentation"
              onKeyDown={() => onClick(country.name, country.code)}
              className="flex flex-row justify-between dark:hover:bg-gun-powder hover:dark:bg-opacity-80 hover:bg-cement-feet hover:bg-opacity-20 rounded-lg cursor-pointer px-3"
              onClick={() => onClick(country.name, country.code)}
            >
              <div className="flex flex-row items-center m-1 gap-3 p-1 cursor-pointer">
                <img
                  src={`./flags/${country.code.toLowerCase()}.svg`}
                  className="h-8"
                  alt={country.code}
                />
                <div className="flex items-center cursor-pointer">
                  {country.name}
                </div>
              </div>
              <div className="p-4 flex items-center text-mercury-mist text-sm cursor-pointer">
                {isSelected(country.code) ? t('selected') : ''}
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
