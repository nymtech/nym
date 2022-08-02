import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { ReceiveModal } from './ReceiveModal';

export const Receive = ({ hasStorybookStyles }: { hasStorybookStyles?: {} }) => {
  const { showReceiveModal, handleShowReceiveModal } = useContext(AppContext);

  if (showReceiveModal)
    return <ReceiveModal onClose={handleShowReceiveModal} open={showReceiveModal} {...hasStorybookStyles} />;

  return null;
};
