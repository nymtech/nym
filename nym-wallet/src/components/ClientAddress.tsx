import React, { FC, useContext } from 'react';
import { Box, Typography, Tooltip } from '@mui/material';
import { ClientContext } from '../context/main';
import { CopyToClipboard } from './CopyToClipboard';
import { splice } from '../utils';

const AddressTooltip: FC<{ visible?: boolean; address?: string; children: React.ReactElement<any, any> }> = ({
  visible,
  address,
  children,
}) => {
  if (!visible) {
    return children;
  }
  if (!address) {
    return children;
  }
  return (
    <Tooltip title={address} arrow>
      {children}
    </Tooltip>
  );
};

type ClientAddressProps = {
  withLabel?: boolean;
  withCopy?: boolean;
  showEntireAddress?: boolean;
};

export const ClientAddressDisplay: FC<ClientAddressProps & { address?: string }> = ({
  withLabel,
  withCopy,
  showEntireAddress,
  address,
}) => (
  <Box>
    {withLabel && (
      <>
        <Typography variant="body2" component="span" sx={{ color: 'grey.600' }}>
          Address:
        </Typography>{' '}
      </>
    )}

    <AddressTooltip address={address} visible={!showEntireAddress}>
      <Typography variant="body2" component="span" color="nym.background.dark" sx={{ mr: 1 }}>
        {showEntireAddress ? address || '' : splice(6, address)}
      </Typography>
    </AddressTooltip>
    {withCopy && <CopyToClipboard text={address} iconButton />}
  </Box>
);

export const ClientAddress: FC<ClientAddressProps> = ({ ...props }) => {
  const { clientDetails } = useContext(ClientContext);
  return <ClientAddressDisplay {...props} address={clientDetails?.client_address} />;
};
