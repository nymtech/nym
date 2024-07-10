import { InfoOutlined } from '@mui/icons-material';
import { Tooltip, TooltipProps } from '@mui/material';

export const InfoTooltip = ({
  title,
  tooltipPlacement = 'bottom',
  light,
  size = 'inherit',
}: {
  title: string;
  tooltipPlacement?: TooltipProps['placement'];
  light?: boolean;
  size?: 'small' | 'medium' | 'large' | 'inherit';
}) => (
  <Tooltip title={title} arrow placement={tooltipPlacement}>
    <InfoOutlined fontSize={size} sx={{ color: light ? 'grey.500' : undefined }} />
  </Tooltip>
);
