import React, { useState } from 'react';
import { Box, Button, ListItemIcon, ListItemText, MenuItem, Stack, Tooltip, Typography } from '@mui/material';
import { DelegationEventKind } from '@nymproject/types';
import { Delegate, Undelegate } from '../../svg-icons';
import { ActionsMenu, ActionsMenuItem } from '../ActionsMenu';
import { DelegateListItemPending } from './types';

export type DelegationListItemActions = 'delegate' | 'undelegate' | 'redeem' | 'compound';

const BUTTON_SIZE = '32px';
const MIN_WIDTH = '150px';

export const DelegationActions: React.FC<{
  onActionClick?: (action: DelegationListItemActions) => void;
  isPending?: DelegateListItemPending;
  disableRedeemingRewards?: boolean;
}> = ({ disableRedeemingRewards, onActionClick, isPending }) => {
  if (isPending) {
    return (
      <Box py={0.5} fontSize="inherit" minWidth={MIN_WIDTH} minHeight={BUTTON_SIZE}>
        <Tooltip title="There will be a new epoch roughly every hour when your changes will take effect" arrow>
          <Typography fontSize="inherit" color="text.disabled">
            Pending {isPending.actionType === 'delegate' ? 'delegation' : 'undelegation'}...
          </Typography>
        </Tooltip>
      </Box>
    );
  }
  return (
    <Stack spacing={2} direction="row" minWidth={MIN_WIDTH}>
      <Tooltip title="Delegate more" arrow>
        <Button
          onClick={() => (onActionClick ? onActionClick('delegate') : undefined)}
          variant="contained"
          disableElevation
          sx={{ maxWidth: BUTTON_SIZE, minWidth: BUTTON_SIZE, height: BUTTON_SIZE, padding: 0 }}
        >
          <Delegate fontSize="small" />
        </Button>
      </Tooltip>
      <Tooltip title="Undelegate" arrow>
        <Button
          variant="outlined"
          sx={{ maxWidth: BUTTON_SIZE, minWidth: BUTTON_SIZE, height: BUTTON_SIZE, padding: 0 }}
          onClick={() => (onActionClick ? onActionClick('undelegate') : undefined)}
        >
          <Undelegate fontSize="small" />
        </Button>
      </Tooltip>
      <Tooltip title={disableRedeemingRewards ? 'There are no rewards to redeem' : 'Redeem rewards'} arrow>
        <span>
          <Button
            disabled={disableRedeemingRewards}
            onClick={() => (onActionClick ? onActionClick('redeem') : undefined)}
            variant="outlined"
            color="secondary"
            sx={{ maxWidth: BUTTON_SIZE, minWidth: BUTTON_SIZE, height: BUTTON_SIZE, padding: 0 }}
          >
            R
          </Button>
        </span>
      </Tooltip>
    </Stack>
  );
};

const DelegationActionsMenuItem = ({
  title,
  description,
  onClick,
  Icon,
  disabled,
}: {
  title: string;
  description?: string;
  onClick?: () => void;
  Icon?: React.ReactNode;
  disabled?: boolean;
}) => (
  <MenuItem sx={{ p: 2 }} onClick={onClick} disabled={disabled}>
    <ListItemIcon sx={{ color: 'text.primary' }}>{Icon}</ListItemIcon>
    <ListItemText sx={{ color: 'text.primary' }} primary={title} secondary={description} />
  </MenuItem>
);

export const DelegationsActionsMenu: React.FC<{
  onActionClick?: (action: DelegationListItemActions) => void;
  isPending?: DelegationEventKind;
  disableRedeemingRewards?: boolean;
  disableCompoundRewards?: boolean;
}> = ({ disableRedeemingRewards, disableCompoundRewards, onActionClick, isPending }) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpenMenu = () => setIsOpen(true);
  const handleOnClose = () => setIsOpen(false);

  const handleActionSelect = (action: DelegationListItemActions) => {
    onActionClick?.(action);
    handleOnClose();
  };

  if (isPending) {
    return (
      <Box py={0.5} fontSize="inherit" minWidth={MIN_WIDTH} minHeight={BUTTON_SIZE}>
        <Tooltip title="There will be a new epoch roughly every hour when your changes will take effect" arrow>
          <Typography fontSize="inherit" color="text.disabled">
            Pending {isPending === 'Delegate' ? 'delegation' : 'undelegation'}...
          </Typography>
        </Tooltip>
      </Box>
    );
  }

  return (
    <ActionsMenu open={isOpen} onOpen={handleOpenMenu} onClose={handleOnClose}>
      <ActionsMenuItem title="Delegate more" Icon={<Delegate />} onClick={() => handleActionSelect('delegate')} />
      <ActionsMenuItem
        title="Undelegate"
        Icon={<Undelegate />}
        onClick={() => handleActionSelect('undelegate')}
        disabled={false}
      />
      <ActionsMenuItem
        title="Redeem"
        description="Trasfer your rewards to your balance"
        Icon={<Typography sx={{ pl: 1 }}>R</Typography>}
        onClick={() => handleActionSelect('redeem')}
        disabled={disableRedeemingRewards}
      />
      <ActionsMenuItem
        title="Compound"
        description="Add your rewards to this delegation"
        Icon={<Typography sx={{ pl: 1 }}>C</Typography>}
        onClick={() => handleActionSelect('compound')}
        disabled={disableCompoundRewards}
      />
    </ActionsMenu>
  );
};
