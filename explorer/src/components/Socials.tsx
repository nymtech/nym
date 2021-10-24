import React from 'react';
import { Box, IconButton } from '@mui/material';
import TelegramIcon from '@mui/icons-material/Telegram';
import GitHubIcon from '@mui/icons-material/GitHub';
import TwitterIcon from '@mui/icons-material/Twitter';

export const Socials: React.FC<{ disableDarkMode?: boolean }> = ({
  disableDarkMode,
}) => (
  <Box>
    <IconButton>
      <TelegramIcon sx={{ color: disableDarkMode ? 'white' : null }} />
    </IconButton>
    <IconButton>
      <GitHubIcon sx={{ color: disableDarkMode ? 'white' : null }} />
    </IconButton>
    <IconButton>
      <TwitterIcon sx={{ color: disableDarkMode ? 'white' : null }} />
    </IconButton>
  </Box>
);

Socials.defaultProps = {
  disableDarkMode: false,
};
