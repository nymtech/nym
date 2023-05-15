import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Stack, IconButton, Typography } from '@mui/material';
import { ArrowDownwardRounded, ArrowUpwardRounded, TollRounded } from '@mui/icons-material';
import { PageLayout } from 'src/layouts/PageLayout';
import { Address, Balance, ReceiveModal } from 'src/components';

type ActionsSchema = Array<{
  title: string;
  Icon: React.ReactNode;
  onClick: () => void;
}>;

const Actions = ({ actionsSchema }: { actionsSchema: ActionsSchema }) => (
  <Box display="flex" justifyContent="space-evenly">
    {actionsSchema.map(({ title, Icon, onClick }) => (
      <Stack justifyContent="center" alignItems="center" key={title}>
        <IconButton color="primary" size="large" onClick={onClick}>
          {Icon}
        </IconButton>
        <Typography>{title}</Typography>
      </Stack>
    ))}
  </Box>
);

export const BalancePage = () => {
  const [showReceiveModal, setShowReceiveModal] = useState(false);
  const navigate = useNavigate();

  const actionsSchema = [
    {
      title: 'Send',
      Icon: <ArrowDownwardRounded fontSize="large" />,
      onClick: () => navigate('/user/send'),
    },
    {
      title: 'Receive',
      Icon: <ArrowUpwardRounded fontSize="large" />,
      onClick: () => setShowReceiveModal(true),
    },
    {
      title: 'Buy',
      Icon: <TollRounded fontSize="large" />,
      onClick: () => navigate('/user/balance'),
    },
  ];

  return (
    <PageLayout>
      <Stack gap={6}>
        <ReceiveModal open={showReceiveModal} onClose={() => setShowReceiveModal(false)} />
        <Address />
        <Balance />
        <Actions actionsSchema={actionsSchema} />
      </Stack>
    </PageLayout>
  );
};
