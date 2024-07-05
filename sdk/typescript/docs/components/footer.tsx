import React from 'react';
import Stack from '@mui/material/Stack';

const links = [
  ['Twitter', 'https://twitter.com/nymproject'],
  ['Telegram', 'https://t.me/nymchan'],
  ['Discord', 'https://discord.gg/FaTJb8q8'],
  ['GitHub', 'https://github.com/nymtech/nym'],
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
