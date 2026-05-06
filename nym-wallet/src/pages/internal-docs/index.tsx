import React, { useContext } from 'react';
import { NymCard } from '../../components';
import { ApiList } from './ApiList';

import { config } from '../../config';
import { AppContext } from '../../context/main';

export const InternalDocs = () => {
  const { isAdminAddress } = useContext(AppContext);

  if (!config.INTERNAL_DOCS_ENABLED || !isAdminAddress) {
    return null;
  }

  return (
    <NymCard title="Docs" subheader="Internal API docs">
      <ApiList />
    </NymCard>
  );
};
