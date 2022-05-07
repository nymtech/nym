import React, { useContext } from 'react';
import { NymCard } from '../../components';
import { ApiList } from './ApiList';

import { ADMIN_ADDRESS, AppContext } from '../../context/main';

export const InternalDocs = () => {
  const { clientDetails } = useContext(AppContext);
  if (clientDetails?.client_address === ADMIN_ADDRESS) {
    return (
      <NymCard title="Docs" subheader="Internal API docs">
        <ApiList />
      </NymCard>
    );
  }

  return null;
};
