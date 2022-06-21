import * as React from 'react';
import { Tooltip as MUITooltip, TooltipProps } from '@mui/material';

export interface CustomTooltipProps {
  textColor?: string;
  bgColor?: string;
  maxWidth?: number;
}

// const tooltipBackGroundColor = '#A0AED1';

export const Tooltip = (props: TooltipProps & CustomTooltipProps) => {
  const { children, title, arrow, id, placement, textColor, bgColor, maxWidth } = props;
  return (
    <MUITooltip
      title={title}
      id={id}
      arrow={arrow}
      placement={placement}
      componentsProps={{
        tooltip: {
          sx: {
            maxWidth: maxWidth,
            background: bgColor,
            color: textColor,
          '& .MuiTooltip-arrow': {
            color: bgColor
          }}},
      }}
    >
      {children}
    </MUITooltip>
  );
};
