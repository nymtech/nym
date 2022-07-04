import * as React from 'react';
import { Card, CardContent, CardHeader, Stack, Typography } from '@mui/material';
import { MixnodeStatus } from '@nymproject/types';
import { CheckCircleOutline, CircleOutlined, PauseCircleOutlined } from '@mui/icons-material';
import { CopyToClipboard } from '../../../components';
import { splice } from '../../../utils';

interface Props {
  children?: React.ReactNode;
  title: string;
  identityKey: string;
  status?: MixnodeStatus;
  action?: React.ReactNode;
}

const IdentityKey = ({ identityKey }: { identityKey: string }) => (
  <Typography variant="body2" component="span" fontWeight={400} sx={{ mr: 1, color: 'text.primary' }}>
    {splice(6, identityKey)}
    <CopyToClipboard text={identityKey} iconButton />
  </Typography>
);

export const getNodeStatus = ({ status }: { status: MixnodeStatus }) => {
  switch (status) {
    case 'active':
      return (
        <Typography display="flex" alignItems="center" sx={{ color: 'success.main' }}>
          <CheckCircleOutline color="success" sx={{ mr: 0.5, fontSize: 13 }} /> Active
        </Typography>
      );
    case 'standby':
      return (
        <Typography display="flex" alignItems="center" sx={{ color: 'info.main' }}>
          <PauseCircleOutlined color="info" sx={{ mr: 0.5, fontSize: 13 }} /> Standby
        </Typography>
      );
    case 'inactive':
      return (
        <Typography display="flex" alignItems="center" sx={{ color: 'nym.text.dark' }}>
          <CircleOutlined sx={{ mr: 0.5, color: 'nym.text.dark', fontSize: 13 }} /> Inactive
        </Typography>
      );
    case 'not_found':
      return (
        <Typography display="flex" alignItems="center" sx={{ color: 'nym.text.dark' }}>
          <CircleOutlined sx={{ mr: 0.5, color: 'nym.text.dark', fontSize: 13 }} /> Not found
        </Typography>
      );
    default:
      return null;
  }
};

const BondedNodeCard = (props: Props) => {
  const { title: rawTitle, identityKey, status: rawStatus, action, children } = props;
  let Title: string | React.ReactNode = (
    <Typography fontSize={20} fontWeight={600}>
      {rawTitle}
    </Typography>
  );
  if (rawStatus) {
    Title = (
      <Stack direction="column" spacing={1.2}>
        {getNodeStatus({ status: rawStatus })}
        <Typography fontSize={20} fontWeight={600}>
          {rawTitle}
        </Typography>
      </Stack>
    );
  }

  return (
    <Card variant="outlined" sx={{ overflow: 'auto', border: 'none', dropShadow: 'none' }}>
      <CardHeader
        title={Title}
        subheader={<IdentityKey identityKey={identityKey} />}
        action={action}
        disableTypography
        sx={{ pb: 0 }}
      />
      <CardContent sx={{ p: 3 }}>{children}</CardContent>
    </Card>
  );
};

export default BondedNodeCard;
