import React, { ReactElement } from 'react';
import { Tooltip as MUITooltip, TooltipComponentsPropsOverrides, TooltipProps } from '@mui/material';

type ValueType<T> = T[keyof T];

type Props = {
  text: string;
  id: string;
  placement?: ValueType<Pick<TooltipProps, 'placement'>>;
  tooltipSx?: TooltipComponentsPropsOverrides;
  children: React.ReactNode;
};

export const Tooltip = ({ text, id, placement, tooltipSx, children }: Props) => (
  <MUITooltip
    title={text}
    id={id}
    placement={placement || 'top-start'}
    componentsProps={{
      tooltip: {
        sx: {
          maxWidth: 200,
          background: (t) => t.palette.nym.networkExplorer.tooltip.background,
          color: (t) => t.palette.nym.networkExplorer.tooltip.color,
          '& .MuiTooltip-arrow': {
            color: (t) => t.palette.nym.networkExplorer.tooltip.background,
          },
        },
        ...tooltipSx,
      },
    }}
    arrow
  >
    {children as ReactElement<any, any>}
  </MUITooltip>
);
