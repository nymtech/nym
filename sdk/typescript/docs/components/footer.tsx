import React from 'react';
import Stack from '@mui/material/Stack';

const links = [
  ['Twitter', 'https://nymtech.net/go/twitter'],
  ['Telegram', 'https://nymtech.net/go/telegram'],
  ['Discord', 'https://nymtech.net/go/discord'],
  ['GitHub', 'https://nymtech.net/go/github/nym'],
  ['Nym Wallet', 'https://nymtech.net/download/wallet'],
  ['Nym Explorer', 'https://explorer.nymtech.net/'],
  ['Nym Blog', 'https://nymtech.medium.com/'],
  ['Nym Shipyard', 'https://shipyard.nymtech.net/'],
];
export const Footer = () => (
  <Stack direction="row" spacing={3}>
    {links.map((link) => (
      <a key={link[1]} href={link[1]}>
        {link[0]}
      </a>
    ))}
  </Stack>
);
