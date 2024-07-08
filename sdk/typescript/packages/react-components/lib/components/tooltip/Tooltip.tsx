import { IconButton, Tooltip as MUITooltip } from '@mui/material';
import InfoOutlinedIcon from '@mui/icons-material/InfoOutlined';

export type CustomTooltipProps = {
  title: string;
  arrow?: boolean;
  id: string;
  placement?:
    | 'bottom'
    | 'left'
    | 'right'
    | 'top'
    | 'bottom-end'
    | 'bottom-start'
    | 'left-end'
    | 'left-start'
    | 'right-end'
    | 'right-start'
    | 'top-end'
    | 'top-start';
  textColor?: string;
  bgColor?: string;
  maxWidth?: number;
};

const TooltipInfoIcon: React.ReactElement<any, any> = (
  <IconButton
    sx={{
      padding: 0,
      py: 1,
      pr: 1,
    }}
    disableFocusRipple
    disableRipple
  >
    <InfoOutlinedIcon
      sx={{
        height: '18px',
        width: '18px',
      }}
    />
  </IconButton>
);

export const Tooltip = (props: CustomTooltipProps) => {
  const { title, arrow, id, placement, textColor, bgColor, maxWidth } = props;
  return (
    <MUITooltip
      title={title}
      id={id}
      arrow={arrow}
      placement={placement}
      componentsProps={{
        tooltip: {
          sx: {
            maxWidth,
            background: bgColor,
            color: textColor,
            '& .MuiTooltip-arrow': {
              color: bgColor,
            },
          },
        },
      }}
    >
      {TooltipInfoIcon}
    </MUITooltip>
  );
};
