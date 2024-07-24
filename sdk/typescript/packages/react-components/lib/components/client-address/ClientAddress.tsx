import { Box, Typography, Tooltip } from '@mui/material';
import { CopyToClipboard } from '../clipboard/CopyToClipboard';

type AddressTooltipProps = { visible?: boolean; address?: string; children: React.ReactNode };

const AddressTooltip = ({ visible, address, children }: AddressTooltipProps) => {
  if (!visible || !address) {
    return <>{children}</>;
  }

  return (
    <Tooltip title={address} arrow>
      <>{children}</>
    </Tooltip>
  );
};

export type ClientAddressProps = {
  address: string;
  withLabel?: boolean;
  withCopy?: boolean;
  smallIcons?: boolean;
  showEntireAddress?: boolean;
};

export const ClientAddress = ({
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
