import { ComponentMeta } from '@storybook/react';
import { Typography } from '@mui/material';
import { Link as LinkIcon } from '@mui/icons-material';
import { Link } from '@lib/components/link';

export default {
  title: 'Basics/Link',
  component: Link,
} as ComponentMeta<typeof Link>;

export const Default = () => <Link text="link" href="https://nymtech.net/" target="_blank" />;

export const NoIcon = () => <Link text="link" href="https://nymtech.net/" target="_blank" noIcon />;

export const WithCustomChildren = () => (
  <Link href="https://nymtech.net/" target="_blank">
    <LinkIcon />
  </Link>
);

export const InTextExample = () => (
  <Typography>
    You can find the Nym website <Link href="https://nymtech.net/" target="_blank" text="here" />.
  </Typography>
);
