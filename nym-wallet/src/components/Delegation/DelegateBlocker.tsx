import React from 'react';
import { SimpleModal } from '../Modals/SimpleModal';

export const OverSaturatedBlockerModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  identityKey?: string;
  header?: string;
  subHeader?: string;
  buttonText?: string;
}> = ({ open, onClose, header, subHeader, buttonText }) => {

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={onClose}
      header={header || 'Delegate'}
      subHeader={subHeader || "This node is over saturated, you can’t compound rewards to it"}
      okLabel={buttonText || 'Close'}
    />
  );
};
