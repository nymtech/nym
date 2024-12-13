import React from "react";
import Stack from "@mui/material/Stack";

const links = [
  ["Matrix", "https://matrix.to/#/#dev:nymtech.chat"],
  ["GitHub", "https://nymtech.net/go/github/nym"],
  ["Nym Wallet", "https://nymtech.net/download/wallet"],
  ["Nym Explorer", "https://explorer.nymtech.net/"],
  ["Nym Blog", "https://nymtech.medium.com/"],
  ["Twitter", "https://nymtech.net/go/x"],
  ["Telegram", "https://nymtech.net/go/telegram"],
];
export const Footer = () => (
  <Stack direction="row" spacing={2}>
    {links.map((link) => (
      <a key={link[1]} href={link[1]}>
        {link[0]}
      </a>
    ))}
  </Stack>
);
