import React from "react";
import Stack from "@mui/material/Stack";

const links = [
  ["Matrix", "https://matrix.to/#/#dev:nymtech.chat"],
  ["GitHub", "https://nym.com/go/github/nym"],
  ["Nym Wallet", "https://nym.com/download/wallet"],
  ["Nym Explorer", "https://explorer.nym.com/"],
  ["Nym Blog", "https://nymtech.medium.com/"],
  ["Twitter", "https://nym.com/go/x"],
  ["Telegram", "https://nym.com/go/telegram"],
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
