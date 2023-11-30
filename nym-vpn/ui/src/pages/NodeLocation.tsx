import {useTranslation} from 'react-i18next';
import {Input} from '@mui/base/Input';
import React, {useState} from "react";

type Props = {
    node: 'entry' | 'exit';
};

const countries = [
    "USA",
    "France",
    "Germany",
    "Switzerland",
    "Mexico",
    "China",
    "South Korea",
    "Japan",
    "Canada"
]

function NodeLocation({node}: Props) {
    const {t} = useTranslation();
    const [search, setSearch] = useState('');
    const [foundCountries, setFoundCountries] = useState(countries);
    type InputEvent = React.ChangeEvent<HTMLInputElement>;
    const filter = (e : InputEvent) => {
        const keyword = e.target.value;
        if (keyword !== '') {
            const results = countries.filter((country) => {
                return country.toLowerCase().startsWith(keyword.toLowerCase());
                // Use the toLowerCase() method to make it case-insensitive
            });
            setFoundCountries(results);
        } else {
            setFoundCountries(countries);
            // If the text field is empty, show all users
        }
        setSearch(keyword);
    };

    return (
        <div>
            {node === 'entry' ? t('fist-hop-selection') : t('last-hop-selection')}
            <div className="h-full flex flex-col p-4">
                <div className="h-70 flex flex-col justify-center items-center gap-y-2">
                    <div className="flex flex-1 items-end">
                        <Input
                            className="dark:bg-baltic-sea"
                            value={search}
                            onChange={filter}
                            startAdornment={
                                <div className="m-8 inline-flex justify-center">
                      <span className="font-icon">
                        search
                      </span>
                                </div>
                            }
                            slotProps={{
                                input: {
                                    className:
                                        "dark:bg-baltic-sea w-80 text-sm font-sans font-normal leading-5 px-3 py-2 rounded-lg shadow-md shadow-slate-100 dark:shadow-slate-900 focus:shadow-outline-purple dark:focus:shadow-outline-purple focus:shadow-lg border border-solid border-slate-300 hover:border-purple-500 dark:hover:border-purple-500 focus:border-purple-500 dark:focus:border-purple-500 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-300 focus-visible:outline-0",
                                },
                            }}
                            aria-label={t("Search country")}
                            placeholder={t("Search country")}
                        />
                    </div>
                    <div className="flex flex-1 items-end">
                        <div>
                            {foundCountries && foundCountries.length > 0 ? (
                                foundCountries.map((country) => (
                                    <li key={t(country)}>
                                        <span>{t(country)}</span>
                                    </li>
                                ))
                            ) : (
                                <p>{t("No results found!")}</p>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}

export default NodeLocation;
