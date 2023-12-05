import * as React from 'react';
import { Tooltip, SxProps } from '@mui/material';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DoneIcon from '@mui/icons-material/Done';
import { useClipboard } from 'use-clipboard-copy';

export const CopyToClipboard: FCWithChildren<{
  value: string;
  tooltip?: React.ReactNode;
  onCopy?: (value: string) => void;
  smallIcons?: boolean;
  sx?: SxProps;
}> = ({ value, tooltip, onCopy, smallIcons, sx }) => {
  const copy = useClipboard();
  const [showConfirmation, setShowConfirmation] = React.useState<boolean>(false);
  const handleCopy = (e: React.MouseEvent<SVGSVGElement>) => {
    e.stopPropagation();
    setShowConfirmation(true);
    copy.copy(value);
    if (onCopy) {
      onCopy(value);
    }
    setTimeout(() => setShowConfirmation(false), 2000);
  };
  return (
    <Tooltip title={tooltip || `Click to copy ${value} to clipboard`}>
      {showConfirmation ? (
        <DoneIcon color="success" sx={sx} fontSize={smallIcons ? 'small' : 'medium'} />
      ) : (
        <ContentCopyIcon
          onClick={handleCopy}
          sx={{ cursor: 'pointer', ...sx }}
          fontSize={smallIcons ? 'small' : 'medium'}
        />
      )}
    </Tooltip>
  );
};
