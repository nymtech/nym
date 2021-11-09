import * as React from 'react';
import { Box, IconButton } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import TelegramIcon from '@mui/icons-material/Telegram';
import GitHubIcon from '@mui/icons-material/GitHub';
import TwitterIcon from '@mui/icons-material/Twitter';
import { GITHUB_LINK, TELEGRAM_LINK, TWITTER_LINK } from 'src/api/constants';

export const Socials: React.FC<{ isFooter?: boolean }> = ({ isFooter }) => {
  const theme = useTheme();
  const color = isFooter
    ? theme.palette.nym.networkExplorer.footer.socialIcons
    : theme.palette.nym.networkExplorer.topNav.socialIcons;
  return (
    <Box>
      <IconButton
        component="a"
        href={TELEGRAM_LINK}
        target="_blank"
        data-testid="telegram"
      >
        <TelegramIcon sx={{ color }} />
      </IconButton>
      <IconButton
        component="a"
        href={GITHUB_LINK}
        target="_blank"
        data-testid="github"
      >
        <GitHubIcon sx={{ color }} />
      </IconButton>
      <IconButton
        component="a"
        href={TWITTER_LINK}
        target="_blank"
        data-testid="twitter"
      >
        <TwitterIcon sx={{ color }} />
      </IconButton>
    </Box>
  );
};

Socials.defaultProps = {
  isFooter: false,
};
