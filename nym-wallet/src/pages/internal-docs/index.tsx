import React, { useContext } from 'react';
import { NymCard } from '../../components';
import { ApiList } from './ApiList';

import { ClientContext } from '../../context/main';

export const InternalDocs = () => {
  const { isAdminAddress } = useContext(ClientContext);

  if (!isAdminAddress) {
    return null;
  }

  return (
    <NymCard title="Docs" subheader="Internal API docs">
      <ApiList />
    </NymCard>
  );
};
