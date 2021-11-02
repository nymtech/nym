import React from 'react';
import { Box, IconButton } from '@mui/material';
import TelegramIcon from '@mui/icons-material/Telegram';
import GitHubIcon from '@mui/icons-material/GitHub';
import TwitterIcon from '@mui/icons-material/Twitter';
import { GITHUB_LINK, TELEGRAM_LINK, TWITTER_LINK } from 'src/api/constants';
import { palette } from 'src';

export const Socials: React.FC<{ disableDarkMode?: boolean }> = ({
  disableDarkMode,
}) => (
  <Box>
    <IconButton component="a" href={TELEGRAM_LINK} target="_blank" data-testid="telegram">
      <TelegramIcon sx={{ color: disableDarkMode ? palette.white : null }} />
    </IconButton>
    <IconButton component="a" href={GITHUB_LINK} target="_blank" data-testid="github">
      <GitHubIcon sx={{ color: disableDarkMode ? palette.white : null }} />
    </IconButton>
    <IconButton component="a" href={TWITTER_LINK} target="_blank" data-testid="twitter">
      <TwitterIcon sx={{ color: disableDarkMode ? palette.white : null }} />
    </IconButton>
  </Box>
);

Socials.defaultProps = {
  disableDarkMode: false,
};
