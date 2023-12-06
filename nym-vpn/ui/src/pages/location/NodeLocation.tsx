import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import { useMainDispatch, useMainState } from '../../contexts';
import { InputEvent, NodeHop } from '../../types/general';
import { Country, StateDispatch } from '../../types';
import { routes } from '../../constants';
import SearchBox from './SearchBox';
import CountryList from './CountryList';
import QuickConnect from './QuickConnect';

function NodeLocation({ type }: NodeHop) {
  const isEntryNodeSelectionScreen = type === 'entry';
  const { t } = useTranslation('nodeLocation');
  const [countries, setCountries] = useState<Country[]>([]);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(false);
  const [foundCountries, setFoundCountries] = useState<Country[]>([]);

  const {
    localAppData: { entryNode, exitNode },
  } = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const navigate = useNavigate();

  useEffect(() => {
    setLoading(true);
    const getNodeCountries = async () => {
      const countries = await invoke<Country[]>('get_node_countries');
      setTimeout(() => {
        setCountries(countries);
        setFoundCountries(countries);
        setLoading(false);
      }, 1000);
    };
    getNodeCountries().catch(console.error);
  }, []);

  const filter = (e: InputEvent) => {
    const keyword = e.target.value;
    if (keyword !== '') {
      const results = countries.filter((country) => {
        return country.name.toLowerCase().startsWith(keyword.toLowerCase());
        // Use the toLowerCase() method to make it case-insensitive
      });
      setFoundCountries(results);
    } else {
      setFoundCountries(countries);
      // If the text field is empty, show all users
    }
    setSearch(keyword);
  };

  const isCountrySelected = (code: string): boolean => {
    return isEntryNodeSelectionScreen
      ? entryNode?.id === code
      : exitNode?.id === code;
  };

  const setNodeSelection = (name: string, code: string) => {
    const nodeType = isEntryNodeSelectionScreen
      ? 'set-entry-node'
      : 'set-exit-node';
    dispatch({ type: nodeType, data: { country: name, id: code } });
  };
  const handleCountrySelection = (name: string, code: string) => {
    setNodeSelection(name, code);
    navigate(routes.root);
  };

  return (
    <div>
      <div className="h-full flex flex-col">
        <div className="h-70 flex flex-col justify-center items-center gap-y-2 p-1">
          <QuickConnect onClick={handleCountrySelection} />
          <SearchBox
            value={search}
            onChange={filter}
            placeholder={t('search-country')}
          />
          <span className="mt-3" />
          {!loading ? (
            <CountryList
              countries={foundCountries}
              onClick={handleCountrySelection}
              isSelected={isCountrySelected}
            />
          ) : (
            <div>{t('loading')}</div>
          )}
        </div>
      </div>
    </div>
  );
}

export default NodeLocation;
