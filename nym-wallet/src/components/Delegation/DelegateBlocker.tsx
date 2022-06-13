import React from 'react';
import { SimpleModal } from '../Modals/SimpleModal';

export const OverSaturatedBlockerModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  identityKey?: string;
  header?: string;
  subHeader?: string;
  buttonText?: string;
}> = ({ open, onClose, header, subHeader, buttonText }) => (
  <SimpleModal
    open={open}
    displayCloseIcon={false}
    displayErrorIcon
    onClose={onClose}
    onOk={onClose}
    header={header || 'Delegate'}
    subHeader={subHeader || 'This node is over saturated, you can’t compound rewards to it'}
    okLabel={buttonText || 'Close'}
    sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}
    titleStyles={{
      width: '100%',
      mb: 3,
      textAlign: 'center',
      color: 'error.main',
      fontSize: 16,
    }}
    descriptionStyles={{ textAlign: 'center', color: 'text.primary', fontSize: 14, fontWeight: 400 }}
  />
);
