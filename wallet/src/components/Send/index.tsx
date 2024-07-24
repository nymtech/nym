import { useContext } from 'react';
import { AppContext } from 'src/context';
import { SendModal } from './SendModal';

export const Send = ({ hasStorybookStyles }: { hasStorybookStyles?: object }) => {
  const { showSendModal, handleShowSendModal } = useContext(AppContext);

  if (showSendModal) return <SendModal onClose={handleShowSendModal} hasStorybookStyles={hasStorybookStyles} />;

  return null;
};
