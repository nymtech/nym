import React from 'react';
import { Box, Typography, Tooltip } from '@mui/material';
import { CopyToClipboard } from '../clipboard/CopyToClipboard';

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
  address: string;
  withLabel?: boolean;
  withCopy?: boolean;
  smallIcons?: boolean;
  showEntireAddress?: boolean;
};

export const ClientAddressDisplay = ({
  withLabel,
  withCopy,
  smallIcons,
  showEntireAddress,
  address,
}: ClientAddressProps & { address?: string }) => (
  <Box>
    {withLabel && (
      <>
        <Typography variant="body2" component="span" sx={{ color: 'text.primary' }}>
          Address:
        </Typography>{' '}
      </>
    )}

    <AddressTooltip address={address} visible={!showEntireAddress}>
      <Typography variant="body2" component="span" sx={{ color: 'text.primary', fontWeight: 400, mr: 0.5 }}>
        {showEntireAddress ? address || '' : `${(address || '').slice(0, 6)}...${address.slice(-6)}`}
      </Typography>
    </AddressTooltip>
    {withCopy && <CopyToClipboard smallIcons={smallIcons} value={address} />}
  </Box>
);

export const ClientAddress = ({ ...props }: ClientAddressProps) => <ClientAddressDisplay {...props} />;
