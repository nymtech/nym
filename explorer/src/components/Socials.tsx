import * as React from 'react';
import { Link } from 'react-router-dom';
import { MainContext } from 'src/context/main';
import { TelegramSVGDark, TelegramSVGLight } from 'src/icons/TelegramSVG';
import { GitHubSVGDark, GitHubSVGLight } from 'src/icons/GitHubSVG';
import { TwitterSVGDark, TwitterSVGLight } from 'src/icons/TwitterSVG';
import * as constants from 'src/api/constants';
import { Badge, List, ListItem } from '@mui/material';

type SocialsProps = {
  disableDarkMode?: boolean;
};

export const Socials = ({ disableDarkMode }: SocialsProps) => {
  const { mode } = React.useContext(MainContext);

  return (
    <>
      <List
        sx={{
          pt: 0,
          pb: 0,
          display: 'flex',
          flexDirection: 'row',
          justifyContent: 'space-evenly',
          alignItems: 'center',
        }}
      >
        <ListItem
          disableGutters
          component={Link}
          to={{ pathname: constants.TELEGRAM_LINK }}
          target="_blank"
          disablePadding
          sx={{ ml: 2, background: 'none' }}
        >
          {disableDarkMode && <TelegramSVGDark />}
          {!disableDarkMode && mode === 'dark' && <TelegramSVGDark />}
          {!disableDarkMode && mode !== 'dark' && <TelegramSVGLight />}
        </ListItem>
        <ListItem
          disableGutters
          component={Link}
          to={{ pathname: constants.TWITTER_LINK }}
          target="_blank"
          disablePadding
          sx={{ ml: 2, background: 'none' }}
        >
          {disableDarkMode && <TwitterSVGDark />}
          {!disableDarkMode && mode === 'dark' && <TwitterSVGDark />}
          {!disableDarkMode && mode !== 'dark' && <TwitterSVGLight />}
        </ListItem>
        <ListItem
          disableGutters
          component={Link}
          to={{ pathname: constants.GITHUB_LINK }}
          target="_blank"
          disablePadding
          sx={{ ml: 2, background: 'none' }}
        >
          {disableDarkMode && <GitHubSVGDark />}
          {!disableDarkMode && mode === 'dark' && <GitHubSVGDark />}
          {!disableDarkMode && mode !== 'dark' && <GitHubSVGLight />}
        </ListItem>
      </List>
    </>
  );
};
