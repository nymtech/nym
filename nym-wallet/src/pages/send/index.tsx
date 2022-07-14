import React, { useContext } from 'react';
import { NymCard } from '../../components';
import { SendWizard } from './SendWizard';
import { AppContext } from '../../context/main';
import { PageLayout } from '../../layouts';

export const Send = () => {
  const { clientDetails } = useContext(AppContext);
  return (
    <PageLayout>
      <NymCard title={`Send ${clientDetails?.mix_denom}`} noPadding>
        <SendWizard />
      </NymCard>
    </PageLayout>
  );
};
