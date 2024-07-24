import { SxProps } from '@mui/material';
import { SimpleModal } from '../Modals/SimpleModal';

export const OverSaturatedBlockerModal: FCWithChildren<{
  open: boolean;
  onClose?: () => void;
  header?: string;
  subHeader?: string;
  buttonText?: string;
  sx?: SxProps;
  backdropProps?: object;
}> = ({ open, onClose, header, subHeader, buttonText, sx, backdropProps }) => (
  <SimpleModal
    open={open}
    hideCloseIcon
    displayErrorIcon
    onClose={onClose}
    onOk={async () => onClose?.()}
    header={header || 'Delegate'}
    subHeader={subHeader || "This node is over saturated, you can't delegate more stake to it"}
    okLabel={buttonText || 'Close'}
    sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', ...sx }}
    headerStyles={{
      width: '100%',
      mb: 3,
      textAlign: 'center',
      color: 'error.main',
      fontSize: 16,
      textTransform: 'capitalize',
    }}
    subHeaderStyles={{ textAlign: 'center', color: 'text.primary', fontSize: 14, fontWeight: 400 }}
    backdropProps={backdropProps}
  />
);
