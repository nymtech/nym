import * as React from 'react';
import Badge from '@mui/material/Badge';
import IconButton from '@mui/material/IconButton';
import { MainContext } from 'src/context/main';
import { TelegramSVGDark, TelegramSVGLight } from 'src/icons/TelegramSVG';
import { GitHubSVGDark, GitHubSVGLight } from 'src/icons/GitHubSVG';
import { TwitterSVGDark, TwitterSVGLight } from 'src/icons/TwitterSVG';
import * as constants from 'src/api/constants';

type SocialsProps = {
  disableDarkMode?: boolean;
};

export const Socials = ({ disableDarkMode }: SocialsProps) => {
  const { mode } = React.useContext(MainContext);

  return (
    <>
      <IconButton size="large">
        <a href={constants.TELEGRAM_LINK} target="_blank" rel="noreferrer">
          <Badge>
            {disableDarkMode && <TelegramSVGDark />}
            {!disableDarkMode && mode === 'dark' && <TelegramSVGDark />}
            {!disableDarkMode && mode !== 'dark' && <TelegramSVGLight />}
          </Badge>
        </a>
      </IconButton>
      <IconButton size="large">
        <a href={constants.TWITTER_LINK} target="_blank" rel="noreferrer">
          <Badge>
            {disableDarkMode && <TwitterSVGDark />}
            {!disableDarkMode && mode === 'dark' && <TwitterSVGDark />}
            {!disableDarkMode && mode !== 'dark' && <TwitterSVGLight />}
          </Badge>
        </a>
      </IconButton>
      <IconButton size="large">
        <a href={constants.GITHUB_LINK} target="_blank" rel="noreferrer">
          <Badge>
            {disableDarkMode && <GitHubSVGDark />}
            {!disableDarkMode && mode === 'dark' && <GitHubSVGDark />}
            {!disableDarkMode && mode !== 'dark' && <GitHubSVGLight />}
          </Badge>
        </a>
      </IconButton>
    </>
  );
};
