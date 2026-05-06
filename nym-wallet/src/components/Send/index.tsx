import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { SendModal } from './SendModal';

export const Send = () => {
  const { showSendModal, handleCloseSendModal } = useContext(AppContext);

  if (showSendModal) return <SendModal onClose={handleCloseSendModal} />;

  return null;
};
