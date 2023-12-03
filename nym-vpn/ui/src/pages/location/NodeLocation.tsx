import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import { Country, StateDispatch } from '../../types';
import SearchBox from './SearchBox.tsx';
import { InputEvent } from '../../types/general.ts';
import { invoke } from '@tauri-apps/api';
import { useMainDispatch, useMainState } from '../../contexts';
import { useNavigate } from 'react-router-dom';
import { routes } from '../../constants.ts';
import CountryList from './CountryList.tsx';
import QuickConnect from './QuickConnect.tsx';

type Props = {
  node: 'entry' | 'exit';
};

function NodeLocation({ node }: Props) {
  const isEntryNodeSelectionScreen = node === 'entry';
  const { t } = useTranslation('nodeLocation');
  const [countries, setCountries] = useState(Array<Country>);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(false);
  const [foundCountries, setFoundCountries] = useState(Array<Country>);

  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const navigate = useNavigate();

  useEffect(() => {
    setLoading(true);
    const getNodeCountries = async () => {
      const countries = await invoke<Array<Country>>('get_node_countries');
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
      ? isCountrySelectedEntryNode(code)
      : isCountrySelectedExitNode(code);
  };

  const isCountrySelectedEntryNode = (code: String): boolean => {
    return state.localAppData.entryNode?.id === code;
  };

  const isCountrySelectedExitNode = (code: String): boolean => {
    return state.localAppData.exitNode?.id === code;
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
