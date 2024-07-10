import * as React from "react";
import { Box, Typography } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { Tooltip } from "@nymproject/react";
import { InfoOutlined } from "@mui/icons-material";

export const CustomColumnHeading: FCWithChildren<{
  headingTitle: string;
  tooltipInfo?: string;
}> = ({ headingTitle, tooltipInfo }) => {
  const theme = useTheme();

  return (
    <Box alignItems="center" display="flex">
      {tooltipInfo && (
        <Tooltip
          title={tooltipInfo}
          id={headingTitle}
          placement="top-start"
          textColor={theme.palette.nym.networkExplorer.tooltip.color}
          bgColor={theme.palette.nym.networkExplorer.tooltip.background}
          maxWidth={230}
          arrow
          TooltipIcon={<InfoOutlined sx={{ width: 18, height: 18, mr: 1 }} />}
        />
      )}
      <Typography variant="body2" fontWeight={600} data-testid={headingTitle}>
        {headingTitle}
      </Typography>
    </Box>
  );
};
