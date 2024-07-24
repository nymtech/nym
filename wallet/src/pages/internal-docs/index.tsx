import { useContext } from 'react';
import { NymCard } from '../../components';
import { ApiList } from './ApiList';

import { AppContext } from '../../context/main';

export const InternalDocs = () => {
  const { isAdminAddress } = useContext(AppContext);

  if (!isAdminAddress) {
    return null;
  }

  return (
    <NymCard title="Docs" subheader="Internal API docs">
      <ApiList />
    </NymCard>
  );
};
