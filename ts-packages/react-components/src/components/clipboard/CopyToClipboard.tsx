import * as React from 'react';
import { useClipboard } from 'use-clipboard-copy';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DoneIcon from '@mui/icons-material/Done';
import { Tooltip } from '@mui/material';
import { SxProps } from '@mui/system';

export const CopyToClipboard: React.FC<{
  value: string;
  tooltip?: React.ReactNode;
  onCopy?: (value: string) => void;
  sx?: SxProps;
}> = ({ value, tooltip, onCopy, sx }) => {
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
        <DoneIcon color="success" sx={sx} />
      ) : (
        <ContentCopyIcon data-testid="ContentCopyIcon" onClick={handleCopy} sx={{ cursor: 'pointer', ...sx }} />
      )}
    </Tooltip>
  );
};
