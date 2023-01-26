import React, { FC, useContext } from 'react';
import { Box, Typography, Tooltip } from '@mui/material';
import { AppContext } from '../context/main';
import { CopyToClipboard } from './CopyToClipboard';
import { splice } from '../utils';

const AddressTooltip: FCWithChildren<{ visible?: boolean; address?: string }> = ({ visible, address, children }) => {
  if (!visible || !address) {
    // eslint-disable-next-line react/jsx-no-useless-fragment
    return <>{children}</>;
  }

  return (
    <Tooltip title={address} arrow>
      {/* eslint-disable-next-line react/jsx-no-useless-fragment */}
      <>{children}</>
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
        <Typography variant="body2" component="span" sx={{ color: 'text.primary' }}>
          Address:
        </Typography>{' '}
      </>
    )}

    <AddressTooltip address={address} visible={!showEntireAddress}>
      <Typography data-testid="wallet-address" variant="body2" component="span" sx={{ mr: 1, color: 'text.primary', fontWeight: 400 }}>
        {showEntireAddress ? address || '' : splice(6, address)}
      </Typography>
    </AddressTooltip>
    {withCopy && <CopyToClipboard text={address} iconButton />}
  </Box>
);

export const ClientAddress: FC<ClientAddressProps> = ({ ...props }) => {
  const { clientDetails } = useContext(AppContext);
  return <ClientAddressDisplay {...props} address={clientDetails?.client_address} />;
};
