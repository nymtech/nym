import React, { useContext } from 'react';
import { NymCard } from '../../components';
import { SendWizard } from './SendWizard';
import { AppContext } from '../../context/main';
import { PageLayout } from '../../layouts';

export const Send = () => {
  const { currency } = useContext(AppContext);
  return (
    <PageLayout>
      <NymCard title={`Send ${currency?.major}`} noPadding>
        <SendWizard />
      </NymCard>
    </PageLayout>
  );
};
