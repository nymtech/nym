import * as React from 'react';
import { Box, IconButton } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { TelegramIcon } from '../icons/socials/TelegramIcon';
import { GitHubIcon } from '../icons/socials/GitHubIcon';
import { TwitterIcon } from '../icons/socials/TwitterIcon';
import { DiscordIcon } from '../icons/socials/DiscordIcon';

// socials
export const TELEGRAM_LINK = 'https://t.me/nymchan';
export const TWITTER_LINK = 'https://twitter.com/nymproject';
export const GITHUB_LINK = 'https://github.com/nymtech';
export const DISCORD_LINK = 'https://discord.gg/FaTJb8q8';

export const Socials: FCWithChildren<{ isFooter?: boolean }> = ({ isFooter }) => {
  const theme = useTheme();
  const color = isFooter
    ? theme.palette.nym.networkExplorer.footer.socialIcons
    : theme.palette.nym.networkExplorer.topNav.socialIcons;
  return (
    <Box>
      <IconButton component="a" href={TELEGRAM_LINK} target="_blank" data-testid="telegram">
        <TelegramIcon color={color} size={24} />
      </IconButton>
      <IconButton component="a" href={DISCORD_LINK} target="_blank" data-testid="discord">
        <DiscordIcon color={color} size={24} />
      </IconButton>
      <IconButton component="a" href={TWITTER_LINK} target="_blank" data-testid="twitter">
        <TwitterIcon color={color} size={24} />
      </IconButton>
      <IconButton component="a" href={GITHUB_LINK} target="_blank" data-testid="github">
        <GitHubIcon color={color} size={24} />
      </IconButton>
    </Box>
  );
};

Socials.defaultProps = {
  isFooter: false,
};
