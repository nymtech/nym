import { Country } from '../../types';
import { useTranslation } from 'react-i18next';
interface CountryListProps {
  countries: Array<Country>;
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
    <ul className="flex flex-col w-full items-stretch p-1">
      {countries && countries.length > 0 ? (
        countries.sort((a, b) => a.name.localeCompare(b.name)).map((country) => (
          <li key={t(country.name)} className="list-none w-full">
            <div
              className="flex flex-row justify-between dark:hover:bg-baltic-sea-jaguar hover:bg-coal-mine-light rounded-lg cursor-pointer"
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
              <div className="p-4 flex items-center text-spanish-gray text-xs cursor-pointer">
                {isSelected(country.code) ? t('selected') : ''}
              </div>
            </div>
          </li>
        ))
      ) : (
        <p>{t('none-found')}</p>
      )}
    </ul>
  );
}
